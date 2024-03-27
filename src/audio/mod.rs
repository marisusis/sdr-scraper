use byteorder::ReadBytesExt;

pub mod ima_adpcm;

pub struct Writer {
    pub wav_writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    pub decoder: ima_adpcm::IMA_ADPCM_Decoder,
}

impl Writer {
    pub fn new(file: std::fs::File) -> Self {
        Writer {
            wav_writer: hound::WavWriter::new(
                std::io::BufWriter::new(file),
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 12000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap(),
            decoder: ima_adpcm::IMA_ADPCM_Decoder::new(),
        }
    }

    pub fn write_sample(&mut self, sample: i16) {
        let mut writer = self.wav_writer.get_i16_writer(1);
        writer.write_sample(sample);
        writer.flush().unwrap();
    }

    pub fn write_samples(&mut self, samples: &Vec<u8>) {
        // log::debug!("Writing samples: {:?}", samples);
        let mut writer = self.wav_writer.get_i16_writer((samples.len() as u32) * 2);
        let mut cursor = std::io::Cursor::new(samples);
        while let Ok(sample) = cursor.read_u8() {
            let decoded = self.decoder.decode((sample & 0x0F) as u16);
            writer.write_sample(decoded);
            let decoded = self.decoder.decode((sample >> 4) as u16);
            writer.write_sample(decoded);
        }
        writer.flush().unwrap();
    }
}
