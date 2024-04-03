use std::time::Instant;

use byteorder::ReadBytesExt;
use chrono::Utc;
use std::boxed::Box;

pub mod ima_adpcm;

pub struct Writer {
    name: String,
    dir: std::path::PathBuf,
    wav_writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
    decoder: ima_adpcm::IMA_ADPCM_Decoder,
    sample_rate: u32,
    start: Instant,
}

impl Writer {
    pub fn new(name: String, dir: &std::path::Path) -> Self {
        Writer {
            name: name,
            dir: dir.to_path_buf(),
            wav_writer: None,
            sample_rate: 12000,
            decoder: ima_adpcm::IMA_ADPCM_Decoder::new(),
            start: Instant::now(),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
    }

    fn open(&mut self, path: &std::path::Path) {
        self.start = Instant::now();
        let file = std::fs::File::create(self.dir.join(path)).unwrap();
        self.wav_writer = Some(
            hound::WavWriter::new(
                std::io::BufWriter::new(file),
                hound::WavSpec {
                    channels: 1,
                    sample_rate: self.sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap(),
        );
        self.decoder = ima_adpcm::IMA_ADPCM_Decoder::new();
    }

    pub fn write_samples(&mut self, samples: &Vec<u8>) {
        if self.wav_writer.is_none() {
            self.open(std::path::Path::new(
                format!("{}_{}.wav", self.name, Utc::now().format("%Y%m%d_%H%M%S")).as_str(),
            ));
        }

        let mut writer = self
            .wav_writer
            .as_mut()
            .unwrap()
            .get_i16_writer((samples.len() as u32) * 2);
        let mut cursor = std::io::Cursor::new(samples);
        while let Ok(sample) = cursor.read_u8() {
            let decoded = self.decoder.decode((sample & 0x0F) as u16);
            writer.write_sample(decoded);
            let decoded = self.decoder.decode((sample >> 4) as u16);
            writer.write_sample(decoded);
        }
        writer.flush().unwrap();
        if self.start.elapsed().as_secs() > 1800 {
            self.close();
        }
    }

    pub fn close(&mut self) {
        match self.wav_writer.take() {
            Some(writer) => {
                writer.finalize().unwrap();
            }
            None => {}
        }
    }
}
