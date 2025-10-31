pub struct DmcChannel {
    pub enabled: bool,
    pub sample_address: u16,
    pub sample_length: u16,
    pub current_address: u16,
    pub bytes_remaining: u16,
    pub sample_buffer: u8,
    pub sample_buffer_empty: bool,
    pub shift_register: u8,
    pub bits_remaining: u8,
    pub silence: bool,
    pub timer_period: u16,
    pub timer_value: u16,
    pub output_level: u8,
    pub irq_enabled: bool,
    pub loop_flag: bool,
}

impl DmcChannel {
    pub fn new() -> Self {
        DmcChannel {
            enabled: false,
            sample_address: 0,
            sample_length: 0,
            current_address: 0,
            bytes_remaining: 0,
            sample_buffer: 0,
            sample_buffer_empty: true,
            shift_register: 0,
            bits_remaining: 8,
            silence: true,
            timer_period: 0,
            timer_value: 0,
            output_level: 0,
            irq_enabled: false,
            loop_flag: false,
        }
    }
    
    pub fn tick_timer<F>(&mut self, cpu_read: &F) 
    where 
        F: Fn(u16) -> u8 
    {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            
            if !self.silence {
                if self.shift_register & 1 != 0 {
                    if self.output_level <= 125 {
                        self.output_level += 2;
                    }
                } else {
                    if self.output_level >= 2 {
                        self.output_level -= 2;
                    }
                }
            }
            
            self.shift_register >>= 1;
            self.bits_remaining -= 1;
            
            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                if self.sample_buffer_empty {
                    self.silence = true;
                } else {
                    self.silence = false;
                    self.shift_register = self.sample_buffer;
                    self.sample_buffer_empty = true;
                }
            }
            
            // Fetch new sample byte if needed
            if self.sample_buffer_empty && self.bytes_remaining > 0 {
                self.sample_buffer = cpu_read(self.current_address);
                self.sample_buffer_empty = false;
                self.current_address = self.current_address.wrapping_add(1);
                if self.current_address == 0 {
                    self.current_address = 0x8000;
                }
                self.bytes_remaining -= 1;
                
                if self.bytes_remaining == 0 {
                    if self.loop_flag {
                        self.current_address = self.sample_address;
                        self.bytes_remaining = self.sample_length;
                    }
                    // Note: IRQ handling would go here but requires CPU reference
                }
            }
        } else {
            self.timer_value -= 1;
        }
    }
    
    pub fn output(&self) -> u8 {
        self.output_level
    }
}

