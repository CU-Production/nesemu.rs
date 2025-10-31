const TRIANGLE_TABLE: [u8; 32] = [
    15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0,
    0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15
];

pub struct TriangleChannel {
    pub enabled: bool,
    pub length_counter_halt: bool,
    pub timer_period: u16,
    pub timer_value: u16,
    pub length_counter: u8,
    pub sequence_pos: u8,
    pub linear_counter: u8,
    pub linear_counter_reload: u8,
    pub linear_counter_reload_flag: bool,
}

impl TriangleChannel {
    pub fn new() -> Self {
        TriangleChannel {
            enabled: false,
            length_counter_halt: false,
            timer_period: 0,
            timer_value: 0,
            length_counter: 0,
            sequence_pos: 0,
            linear_counter: 0,
            linear_counter_reload: 0,
            linear_counter_reload_flag: false,
        }
    }
    
    pub fn tick_timer(&mut self) {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            // Only clock sequencer if both counters are non-zero
            if self.length_counter > 0 && self.linear_counter > 0 {
                self.sequence_pos = (self.sequence_pos + 1) % 32;
            }
        } else {
            self.timer_value -= 1;
        }
    }
    
    pub fn tick_length_counter(&mut self) {
        if !self.length_counter_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    pub fn tick_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        
        if !self.length_counter_halt {
            self.linear_counter_reload_flag = false;
        }
    }
    
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.linear_counter == 0 {
            return 0;
        }
        // Silence ultrasonic frequencies to reduce noise
        if self.timer_period < 2 {
            return 0;
        }
        TRIANGLE_TABLE[self.sequence_pos as usize]
    }
}

