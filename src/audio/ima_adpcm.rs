static IMA_INDEX_TABLE: [i16; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

static IMA_STEP_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

pub struct IMA_ADPCM_Decoder {
    step_index: i16,
    prev_sample: i64,
}

impl IMA_ADPCM_Decoder {
    pub fn new() -> Self {
        IMA_ADPCM_Decoder {
            step_index: 0,
            prev_sample: 0,
        }
    }

    pub fn decode(&mut self, sample: u16) -> i16 {
        let sample = sample as i64;
        let step = IMA_STEP_TABLE[self.step_index as usize] as i64;
        let mut diff = step >> 3;
        // let mut diff = diff + (step >> 2);

        if (sample & 1) != 0 {
            diff += step >> 2;
        }
        if (sample & 2) != 0 {
            diff += step >> 1;
        }
        if (sample & 4) != 0 {
            diff += step;
        }
        if (sample & 8) != 0 {
            diff = -diff;
        }

        // let diff = (2 * sample + 1) * 2 * step / 8;
        // log::debug!("diff: {}", diff);

        self.prev_sample += diff;
        if self.prev_sample > 32767 {
            self.prev_sample = 32767;
        } else if self.prev_sample < -32768 {
            self.prev_sample = -32768;
        }
        self.step_index += IMA_INDEX_TABLE[sample as usize];
        if self.step_index < 0 {
            self.step_index = 0;
        } else if self.step_index > 88 {
            self.step_index = 88;
        }
        self.prev_sample as i16
    }
}
