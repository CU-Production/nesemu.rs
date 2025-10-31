const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

pub struct PulseChannel {
    pub enabled: bool,
    pub length_counter_halt: bool,
    pub constant_volume: bool,
    pub volume: u8,
    pub timer_period: u16,
    pub timer_value: u16,
    pub length_counter: u8,
    pub duty: u8,
    pub duty_pos: u8,
    pub envelope_divider: u8,
    pub envelope_counter: u8,
    pub envelope_volume: u8,
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_divider: u8,
    pub sweep_reload: bool,
    is_pulse1: bool,
}

impl PulseChannel {
    pub fn new(is_pulse1: bool) -> Self {
        PulseChannel {
            enabled: false,
            length_counter_halt: false,
            constant_volume: false,
            volume: 0,
            timer_period: 0,
            timer_value: 0,
            length_counter: 0,
            duty: 0,
            duty_pos: 0,
            envelope_divider: 0,
            envelope_counter: 0,
            envelope_volume: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_divider: 0,
            sweep_reload: false,
            is_pulse1,
        }
    }
    
    pub fn tick_timer(&mut self) {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            self.duty_pos = (self.duty_pos + 1) % 8;
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
    
    pub fn tick_sweep(&mut self) {
        if self.sweep_divider == 0 && self.sweep_enabled {
            let change = self.timer_period >> self.sweep_shift;
            if self.sweep_negate {
                self.timer_period = self.timer_period.wrapping_sub(change);
                if self.is_pulse1 {
                    self.timer_period = self.timer_period.wrapping_sub(1);
                }
            } else {
                self.timer_period = self.timer_period.wrapping_add(change);
            }
        }
        
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
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
        
        if self.timer_period < 8 || self.timer_period > 0x7FF {
            return 0;
        }
        
        if DUTY_TABLE[self.duty as usize][self.duty_pos as usize] == 0 {
            return 0;
        }
        
        self.envelope_volume
    }
}

