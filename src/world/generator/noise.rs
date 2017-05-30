use noise::NoiseModule;
use std::ops::Mul;

pub struct NoiseParameters {
    parameters: Vec<Parameter>
}

struct Parameter {
    scale: f32,
    amplitude: f32,
    min: f32,
    max: f32,
}

impl NoiseParameters {
    pub fn new() -> Self {
        NoiseParameters {
            parameters: vec![]
        }
    }

    pub fn push(mut self, amplitude: f32, wavelength: f32, min: f32, max: f32) -> Self {
        self.parameters.push(Parameter {
            scale: wavelength.recip(),
            amplitude: amplitude,
            min: min,
            max: max
        });
        self
    }

    pub fn generate<'a, I, N>(&self, x: f32, z: f32, mut noise: I) -> f32
        where N: NoiseModule<[f32; 2]> + 'a,
              N::Output: Mul<f32, Output=f32>,
              I: Iterator<Item=&'a N> {
        let mut ret = 0.;
        for p in &self.parameters {
            let n = noise.next().expect("end of noise iterator").get([x * p.scale, z * p.scale]) * p.amplitude;
            ret += n;
            if ret < p.min {
                ret = p.min;
            } else if ret < p.max {
                ret = p.max;
            }
        }
        ret
    }
}