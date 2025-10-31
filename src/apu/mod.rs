pub mod pulse;
pub mod triangle;
pub mod noise;
pub mod dmc;

use pulse::PulseChannel;
use triangle::TriangleChannel;
use noise::NoiseChannel;
use dmc::DmcChannel;

const SAMPLE_BUFFER_SIZE: usize = 4096;
const CYCLES_PER_SAMPLE: f32 = 40.584; // 1789773 / 44100 ≈ 40.584

pub struct APU {
    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,
    
    frame_counter_mode: u8,
    frame_interrupt_inhibit: bool,
    frame_counter: u32,
    cycles: u32,
    
    frame_interrupt: bool,
    
    sample_buffer: Vec<f32>,
    sample_write_pos: usize,
    sample_read_pos: usize,
    
    // Audio filters
    highpass_prev_in: f32,
    highpass_prev_out: f32,
    lowpass_prev_out: f32,
    
    // Accurate sample timing
    sample_accumulator: f32,
}

impl APU {
    pub fn new() -> Self {
        APU {
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            
            frame_counter_mode: 0,
            frame_interrupt_inhibit: false,
            frame_counter: 0,
            cycles: 0,
            
            frame_interrupt: false,
            
            sample_buffer: vec![0.0; SAMPLE_BUFFER_SIZE],
            sample_write_pos: 0,
            sample_read_pos: 0,
            
            highpass_prev_in: 0.0,
            highpass_prev_out: 0.0,
            lowpass_prev_out: 0.0,
            
            sample_accumulator: 0.0,
        }
    }
    
    pub fn tick(&mut self, cpu_read: impl Fn(u16) -> u8) {
        self.cycles += 1;
        
        // Tick frame counter
        self.tick_frame_counter();
        
        // Tick timers at APU rate (CPU rate / 2)
        if self.cycles % 2 == 0 {
            self.pulse1.tick_timer();
            self.pulse2.tick_timer();
            self.noise.tick_timer();
            self.dmc.tick_timer(&cpu_read);
        }
        
        // Triangle runs at CPU rate
        self.triangle.tick_timer();
        
        // Generate audio sample with accurate timing
        self.sample_accumulator += 1.0;
        
        if self.sample_accumulator >= CYCLES_PER_SAMPLE {
            self.sample_accumulator -= CYCLES_PER_SAMPLE;
            
            let p1 = self.pulse1.output();
            let p2 = self.pulse2.output();
            let tri = self.triangle.output();
            let noise = self.noise.output();
            let dmc = self.dmc.output();
            
            let mut sample = mix_audio(p1, p2, tri, noise, dmc);
            
            // Apply filters to reduce noise
            sample = self.apply_highpass_filter(sample);
            sample = self.apply_lowpass_filter(sample);
            
            // Write to buffer (prevent overflow)
            let next_write_pos = (self.sample_write_pos + 1) % SAMPLE_BUFFER_SIZE;
            if next_write_pos != self.sample_read_pos {
                self.sample_buffer[self.sample_write_pos] = sample;
                self.sample_write_pos = next_write_pos;
            }
        }
    }
    
    pub fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut result = 0;
                if self.pulse1.length_counter > 0 { result |= 0x01; }
                if self.pulse2.length_counter > 0 { result |= 0x02; }
                if self.triangle.length_counter > 0 { result |= 0x04; }
                if self.noise.length_counter > 0 { result |= 0x08; }
                if self.dmc.bytes_remaining > 0 { result |= 0x10; }
                if self.frame_interrupt { result |= 0x40; }
                if self.dmc.irq_enabled { result |= 0x80; }
                
                self.frame_interrupt = false;
                result
            }
            _ => 0,
        }
    }
    
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr {
            // Pulse 1
            0x4000 => {
                self.pulse1.duty = (val >> 6) & 0x3;
                self.pulse1.length_counter_halt = (val >> 5) & 0x1 != 0;
                self.pulse1.constant_volume = (val >> 4) & 0x1 != 0;
                self.pulse1.volume = val & 0xf;
            }
            0x4001 => {
                self.pulse1.sweep_enabled = (val >> 7) & 0x1 != 0;
                self.pulse1.sweep_period = (val >> 4) & 0x7;
                self.pulse1.sweep_negate = (val >> 3) & 0x1 != 0;
                self.pulse1.sweep_shift = val & 0x7;
                self.pulse1.sweep_reload = true;
            }
            0x4002 => {
                self.pulse1.timer_period = (self.pulse1.timer_period & 0x700) | val as u16;
            }
            0x4003 => {
                self.pulse1.timer_period = (self.pulse1.timer_period & 0xff) | (((val & 0x7) as u16) << 8);
                self.pulse1.length_counter = LENGTH_TABLE[(val >> 3) as usize];
                self.pulse1.duty_pos = 0;
                self.pulse1.envelope_counter = 15;
            }
            
            // Pulse 2
            0x4004 => {
                self.pulse2.duty = (val >> 6) & 0x3;
                self.pulse2.length_counter_halt = (val >> 5) & 0x1 != 0;
                self.pulse2.constant_volume = (val >> 4) & 0x1 != 0;
                self.pulse2.volume = val & 0xf;
            }
            0x4005 => {
                self.pulse2.sweep_enabled = (val >> 7) & 0x1 != 0;
                self.pulse2.sweep_period = (val >> 4) & 0x7;
                self.pulse2.sweep_negate = (val >> 3) & 0x1 != 0;
                self.pulse2.sweep_shift = val & 0x7;
                self.pulse2.sweep_reload = true;
            }
            0x4006 => {
                self.pulse2.timer_period = (self.pulse2.timer_period & 0x700) | val as u16;
            }
            0x4007 => {
                self.pulse2.timer_period = (self.pulse2.timer_period & 0xff) | (((val & 0x7) as u16) << 8);
                self.pulse2.length_counter = LENGTH_TABLE[(val >> 3) as usize];
                self.pulse2.duty_pos = 0;
                self.pulse2.envelope_counter = 15;
            }
            
            // Triangle
            0x4008 => {
                self.triangle.length_counter_halt = (val >> 7) & 0x1 != 0;
                self.triangle.linear_counter_reload = val & 0x7f;
            }
            0x400A => {
                self.triangle.timer_period = (self.triangle.timer_period & 0x700) | val as u16;
            }
            0x400B => {
                self.triangle.timer_period = (self.triangle.timer_period & 0xff) | (((val & 0x7) as u16) << 8);
                self.triangle.length_counter = LENGTH_TABLE[(val >> 3) as usize];
                self.triangle.linear_counter_reload_flag = true;
            }
            
            // Noise
            0x400C => {
                self.noise.length_counter_halt = (val >> 5) & 0x1 != 0;
                self.noise.constant_volume = (val >> 4) & 0x1 != 0;
                self.noise.volume = val & 0xf;
            }
            0x400E => {
                self.noise.mode = (val >> 7) & 0x1 != 0;
                self.noise.timer_period = NOISE_PERIOD_TABLE[(val & 0xf) as usize];
            }
            0x400F => {
                self.noise.length_counter = LENGTH_TABLE[(val >> 3) as usize];
                self.noise.envelope_counter = 15;
            }
            
            // DMC
            0x4010 => {
                self.dmc.irq_enabled = (val >> 7) & 0x1 != 0;
                self.dmc.loop_flag = (val >> 6) & 0x1 != 0;
                self.dmc.timer_period = DMC_RATE_TABLE[(val & 0xf) as usize];
            }
            0x4011 => {
                self.dmc.output_level = val & 0x7f;
            }
            0x4012 => {
                self.dmc.sample_address = 0xC000 | ((val as u16) << 6);
            }
            0x4013 => {
                self.dmc.sample_length = ((val as u16) << 4) | 1;
            }
            
            // Status
            0x4015 => {
                self.pulse1.enabled = (val & 0x01) != 0;
                self.pulse2.enabled = (val & 0x02) != 0;
                self.triangle.enabled = (val & 0x04) != 0;
                self.noise.enabled = (val & 0x08) != 0;
                self.dmc.enabled = (val & 0x10) != 0;
                
                if !self.pulse1.enabled { self.pulse1.length_counter = 0; }
                if !self.pulse2.enabled { self.pulse2.length_counter = 0; }
                if !self.triangle.enabled { self.triangle.length_counter = 0; }
                if !self.noise.enabled { self.noise.length_counter = 0; }
                
                if !self.dmc.enabled {
                    self.dmc.bytes_remaining = 0;
                } else if self.dmc.bytes_remaining == 0 {
                    self.dmc.current_address = self.dmc.sample_address;
                    self.dmc.bytes_remaining = self.dmc.sample_length;
                }
            }
            
            // Frame counter
            0x4017 => {
                self.frame_counter_mode = (val >> 7) & 0x1;
                self.frame_interrupt_inhibit = (val >> 6) & 0x1 != 0;
                if self.frame_interrupt_inhibit {
                    self.frame_interrupt = false;
                }
                self.frame_counter = 0;
            }
            
            _ => {}
        }
    }
    
    pub fn get_sample(&mut self) -> f32 {
        if self.sample_read_pos != self.sample_write_pos {
            let sample = self.sample_buffer[self.sample_read_pos];
            self.sample_read_pos = (self.sample_read_pos + 1) % SAMPLE_BUFFER_SIZE;
            sample
        } else {
            0.0
        }
    }
    
    pub fn samples_available(&self) -> usize {
        if self.sample_write_pos >= self.sample_read_pos {
            self.sample_write_pos - self.sample_read_pos
        } else {
            SAMPLE_BUFFER_SIZE - self.sample_read_pos + self.sample_write_pos
        }
    }
    
    fn tick_frame_counter(&mut self) {
        let step_cycles = 7457; // ~240Hz
        
        if self.frame_counter_mode == 0 {
            // 4-step mode
            let step = (self.cycles / step_cycles) % 4;
            let cycle_in_step = self.cycles % step_cycles;
            
            if cycle_in_step == 0 {
                // Quarter frame (envelope & triangle linear counter)
                self.pulse1.tick_envelope();
                self.pulse2.tick_envelope();
                self.noise.tick_envelope();
                self.triangle.tick_linear_counter();
                
                if step == 1 || step == 3 {
                    // Half frame (length counter & sweep)
                    self.pulse1.tick_length_counter();
                    self.pulse2.tick_length_counter();
                    self.triangle.tick_length_counter();
                    self.noise.tick_length_counter();
                    
                    self.pulse1.tick_sweep();
                    self.pulse2.tick_sweep();
                }
                
                if step == 3 && !self.frame_interrupt_inhibit {
                    self.frame_interrupt = true;
                }
            }
        } else {
            // 5-step mode
            let step = (self.cycles / step_cycles) % 5;
            let cycle_in_step = self.cycles % step_cycles;
            
            if cycle_in_step == 0 {
                self.pulse1.tick_envelope();
                self.pulse2.tick_envelope();
                self.noise.tick_envelope();
                self.triangle.tick_linear_counter();
                
                if step == 1 || step == 4 {
                    self.pulse1.tick_length_counter();
                    self.pulse2.tick_length_counter();
                    self.triangle.tick_length_counter();
                    self.noise.tick_length_counter();
                    
                    self.pulse1.tick_sweep();
                    self.pulse2.tick_sweep();
                }
            }
        }
    }
    
    fn apply_highpass_filter(&mut self, sample: f32) -> f32 {
        // First-order high-pass filter (90 Hz cutoff)
        const ALPHA: f32 = 0.996;
        
        let output = ALPHA * (self.highpass_prev_out + sample - self.highpass_prev_in);
        self.highpass_prev_in = sample;
        self.highpass_prev_out = output;
        
        output
    }
    
    fn apply_lowpass_filter(&mut self, sample: f32) -> f32 {
        // First-order low-pass filter (14 kHz cutoff)
        const ALPHA: f32 = 0.53;
        
        let output = ALPHA * sample + (1.0 - ALPHA) * self.lowpass_prev_out;
        self.lowpass_prev_out = output;
        
        output
    }
}

// Audio mixing function
fn mix_audio(pulse1: u8, pulse2: u8, triangle: u8, noise: u8, dmc: u8) -> f32 {
    // Check if all channels are silent to avoid unnecessary calculations
    if pulse1 == 0 && pulse2 == 0 && triangle == 0 && noise == 0 && dmc == 0 {
        return 0.0;
    }
    
    let pulse_out = if pulse1 > 0 || pulse2 > 0 {
        95.88 / ((8128.0 / (pulse1 as f32 + pulse2 as f32)) + 100.0)
    } else {
        0.0
    };
    
    let tnd_sum = (triangle as f32 / 8227.0) + (noise as f32 / 12241.0) + (dmc as f32 / 22638.0);
    let tnd_out = if tnd_sum > 0.0 {
        159.79 / ((1.0 / tnd_sum) + 100.0)
    } else {
        0.0
    };
    
    pulse_out + tnd_out
}

// Lookup tables
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
    12,  16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
];

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
];

const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54
];

