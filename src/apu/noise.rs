pub struct NoiseChannel {
    pub enabled: bool,
    pub length_counter_halt: bool,
    pub constant_volume: bool,
    pub volume: u8,
    pub mode: bool,
    pub timer_period: u16,
    pub timer_value: u16,
    pub length_counter: u8,
    pub shift_register: u16,
    pub envelope_divider: u8,
    pub envelope_counter: u8,
    pub envelope_volume: u8,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            enabled: false,
            length_counter_halt: false,
            constant_volume: false,
            volume: 0,
            mode: false,
            timer_period: 0,
            timer_value: 0,
            length_counter: 0,
            shift_register: 1,
            envelope_divider: 0,
            envelope_counter: 0,
            envelope_volume: 0,
        }
    }
    
    pub fn tick_timer(&mut self) {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            
            let feedback = if self.mode {
                ((self.shift_register >> 6) ^ (self.shift_register >> 0)) & 1
            } else {
                ((self.shift_register >> 1) ^ (self.shift_register >> 0)) & 1
            };
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer_value -= 1;
        }
    }
    
    pub fn tick_envelope(&mut self) {
        if self.envelope_divider == 0 {
            self.envelope_divider = self.volume;
            if self.envelope_counter > 0 {
                self.envelope_counter -= 1;
            } else if self.length_counter_halt {
                self.envelope_counter = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
        
        if self.constant_volume {
            self.envelope_volume = self.volume;
        } else {
            self.envelope_volume = self.envelope_counter;
        }
    }
    
    pub fn tick_length_counter(&mut self) {
        if !self.length_counter_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 {
            return 0;
        }
        
        if (self.shift_register & 1) == 1 {
            return 0;
        }
        
        self.envelope_volume
    }
}

