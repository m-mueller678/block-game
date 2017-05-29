use noise::NoiseModule;
use std::ops::Mul;

pub struct NoiseParameters {
    base: f32,
    parameters: Vec<Parameter>
}

struct Parameter {
    scale: f32,
    amplitude: f32,
    min: f32,
    max: f32,
}

impl NoiseParameters {
    pub fn new(base: f32) -> Self {
        NoiseParameters {
            base: base,
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

    pub fn generate<N: NoiseModule<[f32;2]>>(&self, x: f32, z: f32, noise:&N) -> f32
    where N::Output: Mul<f32,Output=f32>{
        let mut ret = self.base;
        for p in &self.parameters {
            let n=noise.get([x*p.scale,z*p.scale])*p.amplitude;
            ret+=n;
            if ret<p.min{
                ret=p.min;
            }else if ret <p.max{
                ret=p.max;
            }
        }
        ret
    }
}