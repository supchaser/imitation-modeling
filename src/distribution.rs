use rand::distr::{Distribution, Uniform};
use rand::Rng;

// RightTriangular - стр-ра для генерации правотреугольного распределения
pub struct RightTriangular {
    left: f64,
    right: f64,
    u: Uniform<f64>
}

impl RightTriangular {
    pub fn new(left: f64, right: f64) -> Self {
        assert!(left<right);

        let uniform = Uniform::new(0.0, 1.0).unwrap();

        return RightTriangular { 
            left: left,
            right: right,
            u: uniform,
        }
    }

    pub fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        let u = self.u.sample(rng);
        return self.left + (self.right - self.left) * u.sqrt()
    }
}

// UniformDistr - равномерное распределение
pub struct UniformDistr {
    u: Uniform<f64>
}

impl UniformDistr {
    pub fn new(min: f64, max: f64) -> Self {
        assert!(min < max);
        
        let uniform = Uniform::new(min, max).unwrap();
        
        return UniformDistr { 
            u: uniform,
        }
    }

    pub fn sample<R: Rng>(&self, rng: &mut R) -> f64 {
        self.u.sample(rng)
    }
}