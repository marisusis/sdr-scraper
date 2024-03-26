mod audio;
mod sdr;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use colored::Colorize;
use futures_util::{SinkExt, StreamExt};
use hound::{WavSpec, WavWriter};
use rand::Rng;
use sdr::{kiwi::TuneMessage, AMTuning, KiwiSDR, Station};
use std::{path::Path, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

use crate::{
    audio::ima_adpcm::IMA_ADPCM_Decoder,
    sdr::kiwi::{
        AgcMessage, LoginMessage, SetCompressionMessage, SetIdentityMessage, SetLocationMessage,
    },
};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    log::info!("Hello!");

    let mut rng = rand::thread_rng();
    let (ws_socket, _) = tokio_tungstenite::connect_async(format!(
        "ws://eekiwisdr.engineering.cwru.edu:8073/kiwi/{}/SND",
        rng.gen_range(0..10000)
    ))
    .await
    .unwrap();

    let (mut write, read) = ws_socket.split();

    let mut writer = WavWriter::create(
        Path::new("./").join("RECORD.wav"),
        WavSpec {
            channels: 1,
            sample_rate: 12000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();

    write
        .send(LoginMessage::new(Some("w8edu".to_string())).into())
        .await
        .unwrap();

    // Create audio init oneshot
    let (audio_init_tx, mut audio_init_rx) = tokio::sync::mpsc::channel::<u32>(1);

    let (snd_tx, mut snd_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(30);

    tokio::spawn(async move {
        println!("{}", "Listening for messages...".blue());
        let read = read;
        let mut tx = audio_init_tx;
        let mut snd_tx = snd_tx;
        let mut decoder = IMA_ADPCM_Decoder::new();
        let mut decoder = Arc::new(Mutex::new(decoder));
        read.for_each(|msg| async {
            let decoder = Arc::clone(&decoder);
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!("Error reading message: {:?}", e);
                    return ();
                }
            };

            match msg {
                Message::Ping(_) => {
                    log::debug!("{}", "Ping!".yellow());
                }
                Message::Binary(data) => {                    let code = String::from_utf8(data[..3].to_vec()).unwrap();
                    match code.as_str() {
                        "MSG" => {

                            if data.len() > 50 {
                                log::debug!(
                                    "Received {} message: {:?}",
                                    "MSG".green(),
                                    String::from_utf8(data[4..50].to_vec()).unwrap()
                                );
                            } else {
                                log::debug!(
                                    "Received {} message: {:?}",
                                    "MSG".green(),
                                    String::from_utf8(data[4..].to_vec()).unwrap()
                                );
                            }


                            let str = String::from_utf8(data[4..].to_vec()).unwrap();
                            if str.starts_with("audio_init") {
                                log::info!("Received audio_init message.");
                                let sample_rate = {
                                    let re = regex::Regex::new(r"audio_init=(\d+)\s+audio_rate=(\d+)\s+sample_rate=([\d.]+)").unwrap();
                                    let caps = re.captures(&str).ok_or("No match found").unwrap();

                                    let audio_init: i16 = caps[1].parse().unwrap();
                                    let audio_rate: f64 = caps[2].parse::<f64>().unwrap().ceil();
                                    let sample_rate: f64 = caps[3].parse().unwrap();
                                    audio_rate as u32
                                };

                                tx.send(sample_rate).await.unwrap();
                            }
                        }
                        "SND" => {
                            log::info!("Received sound message.");
                            let data = data[3..].to_vec();
                            let flags = data[0];
                            let seq = LittleEndian::read_u32(&data[1..5]);
                            let smeter = BigEndian::read_u16(&data[5..7]);

                            let rssi = 0.1 * smeter as f32 - 127.0;
                            log::debug!("RSSI: {}", rssi);

                            let data = data[7..].to_vec();
                            let mut output_vec = Vec::<i16>::with_capacity(data.len() / 2);
                            let mut decoder = decoder.lock().await;
                            let mut cursor = std::io::Cursor::new(data);
                            // while let Ok(b) = cursor.read_u16::<LittleEndian>() {
                            while let Ok(b) = cursor.read_u8() {
                                // output_vec.push(b as i16);
                                let decoded = decoder.decode((b & 0x0F) as u16);
                                output_vec.push(decoded);
                                let decoded = decoder.decode((b >> 4) as u16);
                                output_vec.push(decoded);
                            }

                            match snd_tx.send(output_vec).await {
                                Ok(_) => {},
                                Err(e) => {
                                    log::warn!("Sound buffer full!");
                                }
                            }

                        }
                        _ => {
                            log::info!("Received binary message: {:?}", data);
                        }
                    }
                }
                _ => {
                    log::info!("Received message: {:?}", msg);
                }
            }
        })
        .await;
    });

    log::info!("Waiting for audio init message...");

    let sample_rate = audio_init_rx.recv().await.unwrap();
    log::info!("Sample rate: {}", sample_rate);

    log::info!("SET AR OK in=12000 out=44100");
    write
        .send(Message::Text("SET AR OK in=12000 out=44100".to_string()))
        .await
        .unwrap();

    log::info!("SET squelch=0 param=0.00");
    write
        .send(Message::Text("SET squelch=0 param=0.00".to_string()))
        .await
        .unwrap();

    log::info!("Tuning to 7850 kHz AM...");

    let msg = TuneMessage::new(Station::AM(AMTuning {
        bandwidth: 5000,
        freq: 5000_f64,
    }));
    write.send(msg.into()).await.unwrap();

    let msg = SetIdentityMessage::new("KE8TXG".to_string());
    write.send(msg.into()).await.unwrap();

    let msg = SetLocationMessage::new("Cleveland, OH".to_string());
    write.send(msg.into()).await.unwrap();

    let msg = AgcMessage {
        enabled: false,
        decay: 100,
        hang: 100,
        slope: 100,
        thresh: 100,
        gain: 80,
    };
    write.send(msg.into()).await.unwrap();

    log::info!("Enabling compression...");
    let msg = SetCompressionMessage { enabled: true };
    write.send(msg.into()).await.unwrap();

    loop {
        let data = snd_rx.recv().await.unwrap();
        log::debug!("Received sound data");
        let mut writer = writer.get_i16_writer(data.len() as u32);
        for sample in data {
            writer.write_sample(sample);
        }
        writer.flush().unwrap();
    }
}
