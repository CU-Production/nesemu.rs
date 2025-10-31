use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioOutput {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
}

impl AudioOutput {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or("No output device available")?;
        
        let config = device.default_output_config()?;
        
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();
        
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => Self::build_stream::<f32>(&device, &config.into(), buffer_clone)?,
            cpal::SampleFormat::I16 => Self::build_stream::<i16>(&device, &config.into(), buffer_clone)?,
            cpal::SampleFormat::U16 => Self::build_stream::<u16>(&device, &config.into(), buffer_clone)?,
            _ => return Err("Unsupported sample format".into()),
        };
        
        stream.play()?;
        
        Ok(AudioOutput {
            _stream: stream,
            buffer,
        })
    }
    
    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<Mutex<Vec<f32>>>,
    ) -> Result<Stream, Box<dyn std::error::Error>>
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;
        
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut buffer = buffer.lock().unwrap();
                
                for frame in data.chunks_mut(channels) {
                    let sample: f32 = if buffer.is_empty() {
                        0.0
                    } else {
                        buffer.remove(0)
                    };
                    
                    let value: T = cpal::Sample::from_sample(sample);
                    for sample_out in frame.iter_mut() {
                        *sample_out = value;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;
        
        Ok(stream)
    }
    
    pub fn queue_sample(&mut self, sample: f32) {
        let mut buffer = self.buffer.lock().unwrap();
        // Prevent buffer from growing too large (limit to ~0.5 seconds at 44100 Hz)
        if buffer.len() < 22050 {
            buffer.push(sample);
        }
    }
    
    pub fn queue_samples(&mut self, samples: &[f32]) {
        let mut buffer = self.buffer.lock().unwrap();
        for &sample in samples {
            if buffer.len() < 22050 {
                buffer.push(sample);
            }
        }
    }
}

