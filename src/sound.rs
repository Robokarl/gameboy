use sdl2::audio::AudioSpecDesired;

const OUTPUT_BUFFER_LEN: usize = 4096;
const SWEEP_OVERFLOW: u16 = 2047;

struct ToneSweepChannel {
    dac_enabled: bool,
    enabled: bool,
    waveform: [u8; 8],
    wave_pattern_duty: u8,
    waveform_index: usize,
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_decrease: bool,
    sweep_shift: u8,
    sweep_frequency_shadow: u16,
    sweep_counter: u8,
    envelope_initial_volume: u8,
    envelope_up: bool,
    envelope_period: u8,
    envelope_counter: u8,
    volume: u8,
    frequency: u16,
    timer: u16,
    timer_tick: u16,
    counter_mode: bool,
    length_counter: u8,
    output: u8,
}

impl ToneSweepChannel {
    pub fn new() -> Self {
        ToneSweepChannel {
            dac_enabled: false,
            enabled: false,
            waveform: [0, 0, 0, 0, 0, 0, 0, 1],
            wave_pattern_duty: 0,
            waveform_index: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_decrease: false,
            sweep_shift: 0,
            sweep_frequency_shadow: 0,
            sweep_counter: 0,
            envelope_initial_volume: 0,
            envelope_up: false,
            envelope_period: 0,
            envelope_counter: 0,
            volume: 0,
            frequency: 0,
            timer: 8192,
            timer_tick: 0,
            counter_mode: false,
            length_counter: 64,
            output: 0,
        }
    }

    pub fn read_nrx0(&self) -> u8 {
        let mut result = self.sweep_period << 4;
        result |= if self.sweep_decrease { 0x08 } else { 0x00 };
        result |= self.sweep_shift;
        result | 0x80
    }

    pub fn read_nrx1(&self) -> u8 {
        (self.wave_pattern_duty << 6) | 0x3f
    }

    pub fn read_nrx2(&self) -> u8 {
        let mut result = self.envelope_initial_volume << 4;
        if self.envelope_up { result |= 0x08 };
        result | self.envelope_period
    }

    pub fn read_nrx3(&self) -> u8 {
        0xff
    }

    pub fn read_nrx4(&self) -> u8 {
        if self.counter_mode { 0xff } else { 0xbf }
    }

    pub fn write_nrx0(&mut self, value: u8) {
        self.sweep_period = (value & 0x70) >> 4;
        self.sweep_decrease = value & 0x08 != 0;
        self.sweep_shift = value & 0x07;
    }

    pub fn write_nrx1(&mut self, value: u8) {
        self.wave_pattern_duty = (value & 0xc0) >> 6;
        self.waveform = match self.wave_pattern_duty {
            0 => [0, 0, 0, 0, 0, 0, 0, 1],
            1 => [1, 0, 0, 0, 0, 0, 0, 1],
            2 => [1, 0, 0, 0, 0, 1, 1, 1],
            _ => [0, 1, 1, 1, 1, 1, 1, 0],
        };
        self.length_counter = 64 - (value & 0x3f);
    }

    pub fn write_nrx2(&mut self, value: u8) {
        self.envelope_initial_volume = (value & 0xf0) >> 4;
        self.envelope_up = value & 0x08 != 0;
        self.envelope_period = value & 0x07;
        self.dac_enabled = self.envelope_initial_volume != 0 || self.envelope_up;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nrx3(&mut self, value: u8) {
        self.frequency = (self.frequency & 0xff00) | value as u16;
        self.timer = (2048 - self.frequency) * 4;
    }

    pub fn write_nrx4(&mut self, value: u8) {
        self.counter_mode = value & 0x40 != 0;
        self.frequency = (self.frequency & 0x00ff) | ((value as u16 & 0x07) << 8);
        self.timer = (2048 - self.frequency) * 4;

        if value & 0x80 != 0 {
            if self.length_counter == 0 {
                self.length_counter = 64;
            }
            if self.dac_enabled {
                self.enabled = true;
            }
            self.timer_tick = 0;

            self.sweep_frequency_shadow = self.frequency;
            self.sweep_counter = self.sweep_period;
            self.sweep_enabled = self.sweep_period != 0 || self.sweep_shift != 0;
            if self.sweep_shift != 0 && self.next_sweep_frequency() > SWEEP_OVERFLOW {
                self.enabled = false;
            }

            self.envelope_counter = self.envelope_period;
            self.volume = self.envelope_initial_volume;
            self.output = self.waveform[self.waveform_index] * self.volume;
        }

    }

    pub fn next_sweep_frequency(&mut self) -> u16 {
        if self.sweep_decrease {
            self.sweep_frequency_shadow - (self.sweep_frequency_shadow >> self.sweep_shift)
        } else {
            self.sweep_frequency_shadow + (self.sweep_frequency_shadow >> self.sweep_shift)
        }
    }

    pub fn execute_cycle(&mut self) {
        if !self.enabled {
            self.output = 0;
        } else {
            self.timer_tick += 1;
            if self.timer_tick >= self.timer {
                self.timer_tick = 0;
                self.waveform_index = (self.waveform_index + 1) % 8;
                self.output = self.waveform[self.waveform_index] * self.volume;
            }
        }
    }

    pub fn tick_length_counter(&mut self) {
        if self.counter_mode {
            self.length_counter = self.length_counter.saturating_sub(1);
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn tick_envelope_counter(&mut self) {
        if self.envelope_period == 0 {
            return;
        }

        self.envelope_counter = self.envelope_counter.saturating_sub(1);
        if self.envelope_counter == 0 {
            self.envelope_counter = self.envelope_period;
            if self.envelope_up && self.volume != 15 {
                self.volume += 1;
            } else if !self.envelope_up && self.volume != 0 {
                self.volume -= 1;
            }
        }
    }

    pub fn tick_sweep_counter(&mut self) {
        self.sweep_counter = self.sweep_counter.saturating_sub(1);
        if self.sweep_enabled && self.sweep_period != 0 && self.sweep_counter == 0 {
            self.sweep_counter = self.sweep_period;
            let next_sweep_frequency = self.next_sweep_frequency();
            if next_sweep_frequency > SWEEP_OVERFLOW {
                self.enabled = false;
            } else if self.sweep_shift != 0 {
                self.sweep_frequency_shadow = next_sweep_frequency;
                self.frequency = next_sweep_frequency;
                self.timer = (2048 - self.frequency) * 4;
                if self.next_sweep_frequency() > SWEEP_OVERFLOW {
                    self.enabled = false;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.dac_enabled = false;
        self.enabled = false;
        self.waveform = [0, 0, 0, 0, 0, 0, 0, 1];
        self.wave_pattern_duty = 0;
        self.waveform_index = 0;
        self.sweep_enabled = false;
        self.sweep_period = 0;
        self.sweep_decrease = false;
        self.sweep_shift = 0;
        self.sweep_frequency_shadow = 0;
        self.sweep_counter = 0;
        self.envelope_initial_volume = 0;
        self.envelope_up = false;
        self.envelope_period = 0;
        self.envelope_counter = 0;
        self.volume = 0;
        self.frequency = 0;
        self.timer = 8192;
        self.timer_tick = 0;
        self.counter_mode = false;
        self.length_counter = 64;
        self.output = 0;
    }
}



struct WaveChannel {
    enabled: bool,
    dac_enabled: bool,
    output_level: u8,
    frequency: u16,
    timer: u16,
    timer_tick: u16,
    counter_mode: bool,
    length_counter: u16,
    wave_ram: [u8; 32],
    wave_ram_index: usize,
    output: u8,
}

impl WaveChannel {
    pub fn new() -> Self {
        WaveChannel {
            enabled: false,
            dac_enabled: false,
            output_level: 0,
            frequency: 0,
            timer: 4096,
            timer_tick: 0,
            counter_mode: false,
            length_counter: 256,
            wave_ram: [0; 32],
            wave_ram_index: 0,
            output: 0,
        }
    }

    pub fn read_nr30(&self) -> u8 {
        if self.dac_enabled { 0xff } else { 0x7f }
    }

    pub fn read_nr31(&self) -> u8 {
        0xff
    }

    pub fn read_nr32(&self) -> u8 {
        (self.output_level << 5) | 0x9f
    }

    pub fn read_nr33(&self) -> u8 {
        0xff
    }

    pub fn read_nr34(&self) -> u8 {
        if self.counter_mode { 0xff } else { 0xbf }
    }

    pub fn read_wave_ram(&self, index: usize) -> u8 {
        let ram_index = 2 * index;
        let high_nibble = self.wave_ram[ram_index];
        let low_nibble = self.wave_ram[ram_index + 1];
        (high_nibble << 4) | low_nibble
    }

    pub fn write_nr30(&mut self, value: u8) {
        self.dac_enabled = value & 0x80 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nr31(&mut self, value: u8) {
        self.length_counter = 256 - value as u16;
    }

    pub fn write_nr32(&mut self, value: u8) {
        self.output_level = (value & 0x60) >> 5
    }

    pub fn write_nr33(&mut self, value: u8) {
        self.frequency = (self.frequency & 0xff00) | value as u16;
        self.timer = (2048 - self.frequency) * 2;
    }

    pub fn write_nr34(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x00ff) | ((value as u16 & 0x07) << 8);
        self.timer = (2048 - self.frequency) * 2;

        if value & 0x80 != 0 {
            if self.length_counter == 0 {
                self.length_counter = 256;
            }
            if self.dac_enabled {
                self.enabled = true;
            }
            self.timer_tick = 0;
            self.wave_ram_index = 0;
            self.output = self.wave_ram[0];
        }

        self.counter_mode = value & 0x40 != 0;
    }

    pub fn write_wave_ram(&mut self, index: usize, value: u8) {
        let ram_index = 2 * index;
        self.wave_ram[ram_index] = value >> 4;
        self.wave_ram[ram_index + 1] = value & 0x0f;
    }

    pub fn execute_cycle(&mut self) {
        if !self.enabled || self.output_level == 0 {
            self.output = 0;
        } else {
            self.timer_tick += 1;
            if self.timer_tick == self.timer {
                self.timer_tick = 0;
                self.wave_ram_index = (self.wave_ram_index + 1) % 32;
                self.output = self.wave_ram[self.wave_ram_index] >> (self.output_level - 1);
            }
        }
    }

    pub fn tick_length_counter(&mut self) {
        if self.counter_mode {
            self.length_counter = self.length_counter.saturating_sub(1);
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn reset(&mut self) {
        self.enabled = false;
        self.dac_enabled = false;
        self.output_level = 0;
        self.frequency = 0;
        self.timer = 4096;
        self.timer_tick = 0;
        self.counter_mode = false;
        self.length_counter = 256;
        self.wave_ram_index = 0;
        self.output = 0;
    }
}

struct NoiseChannel {
    dac_enabled: bool,
    enabled: bool,
    lfsr: u16,
    lfsr_7_bits: bool,
    shift_clock_frequency: u8,
    frequency_divider_ratio: u8,
    envelope_initial_volume: u8,
    envelope_up: bool,
    envelope_period: u8,
    envelope_counter: u8,
    volume: u8,
    timer: u16,
    timer_tick: u16,
    counter_mode: bool,
    length_counter: u8,
    output: u8,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            dac_enabled: false,
            enabled: false,
            lfsr: 0xffff,
            lfsr_7_bits: false,
            shift_clock_frequency: 0,
            frequency_divider_ratio: 0,
            envelope_initial_volume: 0,
            envelope_up: false,
            envelope_period: 0,
            envelope_counter: 0,
            volume: 0,
            timer: 8,
            timer_tick: 0,
            counter_mode: false,
            length_counter: 64,
            output: 0,
        }
    }

    pub fn read_nr41(&self) -> u8 {
        0xff
    }

    pub fn read_nr42(&self) -> u8 {
        let mut result = self.envelope_initial_volume << 4;
        if self.envelope_up { result |= 0x08 };
        result | self.envelope_period
    }

    pub fn read_nr43(&self) -> u8 {
        let mut result = self.shift_clock_frequency << 4;
        result |= if self.lfsr_7_bits { 0x08 } else { 0x00 };
        result | self.frequency_divider_ratio
    }

    pub fn read_nr44(&self) -> u8 {
        if self.counter_mode { 0xff } else { 0xbf }
    }

    pub fn write_nr41(&mut self, value: u8) {
        self.length_counter = 64 - (value & 0x3f);
    }

    pub fn write_nr42(&mut self, value: u8) {
        self.envelope_initial_volume = (value & 0xf0) >> 4;
        self.envelope_up = value & 0x08 != 0;
        self.envelope_period = value & 0x07;
        self.dac_enabled = self.envelope_initial_volume != 0 || self.envelope_up;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nr43(&mut self, value: u8) {
        self.shift_clock_frequency = (value & 0xf0) >> 4;
        self.lfsr_7_bits = value & 0x08 != 0;
        self.frequency_divider_ratio = value & 0x07;
        let divisor = if self.frequency_divider_ratio == 0 { 
            8
        } else { 
            self.frequency_divider_ratio * 16
        };
        self.timer = (divisor as u16) << self.shift_clock_frequency as u16;
    }

    pub fn write_nr44(&mut self, value: u8) {
        if value & 0x80 != 0 {
            if self.length_counter == 0 {
                self.length_counter = 64;
            }
            if self.dac_enabled {
                self.enabled = true;
            }
            self.timer_tick = 0;

            self.envelope_counter = self.envelope_period;
            self.volume = self.envelope_initial_volume;
            self.lfsr = 0xffff;
            self.output = 0;
        }

        self.counter_mode = value & 0x40 != 0;
    }

    pub fn execute_cycle(&mut self) {
        if !self.enabled {
            self.output = 0;
        } else {
            self.timer_tick += 1;
            if self.timer_tick >= self.timer {
                self.timer_tick = 0;
                let xor_output = ((self.lfsr & 0x0002) >> 1) ^ (self.lfsr & 0x0001);
                self.lfsr = (self.lfsr >> 1) | (xor_output << 14);
                if self.lfsr_7_bits {
                    self.lfsr = (self.lfsr & !0x0040) | (xor_output << 6);
                }
                let lfsr_bit = (!self.lfsr) & 0x0001;
                self.output = lfsr_bit as u8 * self.volume;
            }
        }
    }

    pub fn tick_length_counter(&mut self) {
        if self.counter_mode {
            self.length_counter = self.length_counter.saturating_sub(1);
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn tick_envelope_counter(&mut self) {
        if self.envelope_period == 0 {
            return;
        }

        self.envelope_counter = self.envelope_counter.saturating_sub(1);
        if self.envelope_counter == 0 {
            self.envelope_counter = self.envelope_period;
            if self.envelope_up && self.volume != 15 {
                self.volume += 1;
            } else if !self.envelope_up && self.volume != 0 {
                self.volume -= 1;
            }
        }
    }

    pub fn reset(&mut self) {
        self.dac_enabled = false;
        self.enabled = false;
        self.lfsr = 0xffff;
        self.lfsr_7_bits = false;
        self.shift_clock_frequency = 0;
        self.frequency_divider_ratio = 0;
        self.envelope_initial_volume = 0;
        self.envelope_up = false;
        self.envelope_period = 0;
        self.envelope_counter = 0;
        self.volume = 0;
        self.timer = 8;
        self.timer_tick = 0;
        self.counter_mode = false;
        self.length_counter = 64;
        self.output = 0;
    }
}

pub struct SoundController {
    master_enable: bool,
    audio_queue: sdl2::audio::AudioQueue<f32>,
    output_terminal_settings: u8,
    tone_sweep_channel: ToneSweepChannel,
    tone_channel: ToneSweepChannel,
    wave_channel: WaveChannel,
    noise_channel: NoiseChannel,
    output_buffer: [f32; OUTPUT_BUFFER_LEN],
    left_vin: bool,
    right_vin: bool,
    left_volume: u8,
    right_volume: u8,
    buffer_insert_index: usize,
    buffer_remove_index: usize,
    buffer_size: usize,
    frame_sequence_count: u8,
    tick: u8,
}

impl SoundController {
    pub fn new(sdl: &sdl2::Sdl) -> Self {

        let sdl_audio = sdl.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(93207),
            channels: Some(2),  // stereo
            samples: Some(1024),
        };

        let audio_queue = sdl_audio.open_queue(None, &desired_spec).unwrap();

        // Start playback
        audio_queue.resume();

        SoundController {
            master_enable: false,
            audio_queue,
            output_terminal_settings: 0,
            tone_sweep_channel: ToneSweepChannel::new(),
            tone_channel: ToneSweepChannel::new(),
            wave_channel: WaveChannel::new(),
            noise_channel: NoiseChannel::new(),
            output_buffer: [0.0; OUTPUT_BUFFER_LEN],
            left_vin: false,
            right_vin: false,
            left_volume: 0,
            right_volume: 0,
            buffer_insert_index: 0,
            buffer_remove_index: 0,
            buffer_size: 0,
            frame_sequence_count: 0,
            tick: 0,
        }
    }

    pub fn read(&self, address: usize) -> u8 {
        match address {
            0xff10 => self.tone_sweep_channel.read_nrx0(),
            0xff11 => self.tone_sweep_channel.read_nrx1(),
            0xff12 => self.tone_sweep_channel.read_nrx2(),
            0xff13 => self.tone_sweep_channel.read_nrx3(),
            0xff14 => self.tone_sweep_channel.read_nrx4(),
            0xff16 => self.tone_channel.read_nrx1(),
            0xff17 => self.tone_channel.read_nrx2(),
            0xff18 => self.tone_channel.read_nrx3(),
            0xff19 => self.tone_channel.read_nrx4(),
            0xff1a => self.wave_channel.read_nr30(),
            0xff1b => self.wave_channel.read_nr31(),
            0xff1c => self.wave_channel.read_nr32(),
            0xff1d => self.wave_channel.read_nr33(),
            0xff1e => self.wave_channel.read_nr34(),
            0xff20 => self.noise_channel.read_nr41(),
            0xff21 => self.noise_channel.read_nr42(),
            0xff22 => self.noise_channel.read_nr43(),
            0xff23 => self.noise_channel.read_nr44(),
            0xff24 => {
                let mut result = if self.left_vin { 0x80 } else { 0x00 };
                result |= self.left_volume << 4;
                result |= if self.right_vin { 0x08 } else { 0x00 };
                result | self.right_volume
            }
            0xff25 => self.output_terminal_settings,
            0xff26 => {
                let mut result = if self.master_enable { 0x80 } else { 0x00 };
                if self.noise_channel.enabled { result |= 0x08 };
                if self.wave_channel.enabled { result |= 0x04 };
                if self.tone_channel.enabled { result |= 0x02 };
                if self.tone_sweep_channel.enabled { result |= 0x01 };
                result | 0x70
            }
            0xff30..=0xff3f => self.wave_channel.read_wave_ram(address & 0x000f),
            _ => 0xff
        }
    }

    pub fn write(&mut self, address: usize, value: u8) {
        match address {
            0xff10 if self.master_enable => self.tone_sweep_channel.write_nrx0(value),
            0xff11 if self.master_enable => self.tone_sweep_channel.write_nrx1(value),
            0xff12 if self.master_enable => self.tone_sweep_channel.write_nrx2(value),
            0xff13 if self.master_enable => self.tone_sweep_channel.write_nrx3(value),
            0xff14 if self.master_enable => self.tone_sweep_channel.write_nrx4(value),
            0xff16 if self.master_enable => self.tone_channel.write_nrx1(value),
            0xff17 if self.master_enable => self.tone_channel.write_nrx2(value),
            0xff18 if self.master_enable => self.tone_channel.write_nrx3(value),
            0xff19 if self.master_enable => self.tone_channel.write_nrx4(value),
            0xff1a if self.master_enable => self.wave_channel.write_nr30(value),
            0xff1b if self.master_enable => self.wave_channel.write_nr31(value),
            0xff1c if self.master_enable => self.wave_channel.write_nr32(value),
            0xff1d if self.master_enable => self.wave_channel.write_nr33(value),
            0xff1e if self.master_enable => self.wave_channel.write_nr34(value),
            0xff20 if self.master_enable => self.noise_channel.write_nr41(value),
            0xff21 if self.master_enable => self.noise_channel.write_nr42(value),
            0xff22 if self.master_enable => self.noise_channel.write_nr43(value),
            0xff23 if self.master_enable => self.noise_channel.write_nr44(value),
            0xff24 if self.master_enable => {
                self.left_vin = value & 0x80 != 0x00;
                self.right_vin = value & 0x08 != 0x00;
                self.left_volume = (value & 0x70) >> 4;
                self.right_volume = value & 0x07;
            }
            0xff25 if self.master_enable => self.output_terminal_settings = value,
            0xff26 => {
                self.master_enable = value & 0x80 != 0;
                if !self.master_enable {
                    self.reset();
                }
            }
            0xff30..=0xff3f => self.wave_channel.write_wave_ram(address & 0x000f, value),
            _ => {}
        }
    }
    
    pub fn execute_cycle(&mut self) {

        if self.master_enable {
            self.tone_sweep_channel.execute_cycle();
            self.tone_channel.execute_cycle();
            self.wave_channel.execute_cycle();
            self.noise_channel.execute_cycle();
        }

        if self.tick == 0 {
            let noise_output = self.noise_channel.output as f32 / 100.0;
            let wave_output = self.wave_channel.output as f32 / 100.0;
            let tone_output = self.tone_channel.output as f32 / 100.0;
            let tone_sweep_output = self.tone_sweep_channel.output as f32 / 100.0;

            let mut left = 0.0;
            if self.output_terminal_settings & 0x80 != 0 { left += noise_output };
            if self.output_terminal_settings & 0x40 != 0 { left += wave_output };
            if self.output_terminal_settings & 0x20 != 0 { left += tone_output };
            if self.output_terminal_settings & 0x10 != 0 { left += tone_sweep_output };

            let mut right = 0.0;
            if self.output_terminal_settings & 0x08 != 0 { right += noise_output };
            if self.output_terminal_settings & 0x04 != 0 { right += wave_output };
            if self.output_terminal_settings & 0x02 != 0 { right += tone_output };
            if self.output_terminal_settings & 0x01 != 0 { right += tone_sweep_output };

            self.output_buffer[self.buffer_insert_index] = left * self.left_volume as f32 / 8.0;
            self.output_buffer[self.buffer_insert_index+1] = right * self.right_volume as f32 / 8.0;
            self.buffer_insert_index = (self.buffer_insert_index + 2) % OUTPUT_BUFFER_LEN;
            self.buffer_size += 2;
        }

        self.tick = (self.tick + 1) % 45;
    }

    pub fn tick_frame_sequencer(&mut self) {
        self.frame_sequence_count = (self.frame_sequence_count + 1) % 8;
        if self.frame_sequence_count % 2 == 0 {
            self.tone_sweep_channel.tick_length_counter();
            self.tone_channel.tick_length_counter();
            self.wave_channel.tick_length_counter();
            self.noise_channel.tick_length_counter();
        }
        if self.frame_sequence_count % 4 == 2 {
            self.tone_sweep_channel.tick_sweep_counter();
        }
        if self.frame_sequence_count == 7 {
            self.tone_sweep_channel.tick_envelope_counter();
            self.tone_channel.tick_envelope_counter();
            self.noise_channel.tick_envelope_counter();
        }
    }

    pub fn queue_audio(&mut self) {
        let queue_insert_len = 512;
        if self.audio_queue.size() / 8 < 4096 * 3 && self.buffer_size >= queue_insert_len {
            let start_index = self.buffer_remove_index;
            let end_index = self.buffer_remove_index + queue_insert_len;
            self.audio_queue.queue(&self.output_buffer[start_index..end_index]);
            self.buffer_remove_index = (self.buffer_remove_index + queue_insert_len) % OUTPUT_BUFFER_LEN;
            self.buffer_size -= queue_insert_len;
        }
    }

    pub fn buffer_full(&self) -> bool {
        self.buffer_size == self.output_buffer.len()
    }

    fn reset (&mut self) {
        self.left_vin = false;
        self.right_vin = false;
        self.left_volume = 0;
        self.right_volume = 0;
        self.output_terminal_settings = 0;
        self.tone_sweep_channel.reset();
        self.tone_channel.reset();
        self.wave_channel.reset();
        self.noise_channel.reset();
    }
}

