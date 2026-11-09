//! Speed-responsive combustion and electric motor loops.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Voice { Petrol, Sport, Hybrid, Electric }

impl Voice {
    pub fn for_name(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        if name.contains("tesla") || name.contains("cybertruck") { Self::Electric }
        else if name.contains("i8") { Self::Hybrid }
        else if name.contains("mclaren") || name.contains("porsche") { Self::Sport }
        else { Self::Petrol }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mix {
    pub petrol_position: f32,
    pub petrol_gain: f32,
    pub motor_position: f32,
    pub motor_gain: f32,
}

pub fn driving_mix(voice: Voice, speed: f32, throttle: bool) -> Mix {
    let speed = if speed.is_finite() { speed.abs().min(65.0) } else { 0.0 };
    let load = if throttle { 1.0 } else { 0.0 };
    let gear = (speed / 11.0).floor().min(4.0);
    let rpm = (0.12 + (speed - gear * 11.0) / 11.0 * 0.62 + load * 0.20).clamp(0.0, 1.0);
    let mut mix = Mix {
        petrol_position: (0.2 + rpm * 6.0).clamp(0.0, 7.0),
        petrol_gain: (0.14 + speed / 65.0 * 0.30 + load * 0.19).min(0.72),
        motor_position: (speed / 45.0 * 6.4 + load * 0.5).clamp(0.0, 7.0),
        motor_gain: 0.0,
    };
    match voice {
        Voice::Petrol => {},
        Voice::Sport => { mix.petrol_position = (mix.petrol_position + 0.5).min(7.0); mix.petrol_gain *= 1.15; },
        Voice::Hybrid => { mix.petrol_gain *= 0.72; mix.motor_gain = 0.04 + load * 0.10 + speed / 65.0 * 0.08; },
        Voice::Electric => { mix.petrol_gain = 0.0; mix.motor_gain = (0.025 + speed / 65.0 * 0.37 + load * 0.13).min(0.65); },
    }
    mix
}

pub fn bank_gains(position: f32, gain: f32) -> [f32; 8] {
    let position = position.clamp(0.0, 7.0);
    let lo = position.floor() as usize;
    let hi = (lo + 1).min(7);
    let t = position - lo as f32;
    let mut gains = [0.0; 8];
    gains[lo] += gain * (1.0 - t);
    gains[hi] += gain * t;
    gains
}

#[cfg(native_audio)]
pub struct DriveBanks {
    sounds: Vec<macroquad::audio::Sound>,
    active: [bool; 16],
    positions: [f32; 2],
}

#[cfg(native_audio)]
impl DriveBanks {
    pub async fn load(base: &str) -> Option<Self> {
        let mut sounds = Vec::new();
        for bank in ["engine", "motor"] {
            for level in 0..8 {
                let path = format!("{base}/sounds/{bank}-rpm-{level}.wav");
                let data = match macroquad::file::load_file(&path).await {
                    Ok(data) => data,
                    Err(e) => { eprintln!("Driving sound bank missing ({path}): {e}"); return None; },
                };
                match crate::assets::decode_sound(&data).await {
                    Ok(sound) => sounds.push(sound),
                    Err(e) => { eprintln!("Cannot decode driving sound ({path}): {e}"); return None; },
                }
            }
        }
        Some(Self { sounds, active: [false; 16], positions: [0.0; 2] })
    }

    pub fn stop(&mut self) {
        for i in 0..16 {
            if self.active[i] { macroquad::audio::stop_sound(&self.sounds[i]); self.active[i] = false; }
        }
        self.positions = [0.0; 2];
    }

    pub fn update(&mut self, live: bool, mix: Mix, dt: f32) {
        use macroquad::audio::{play_sound, set_sound_volume, stop_sound, PlaySoundParams};
        if !live { self.stop(); return; }
        let smooth = (dt.max(0.0) * 8.0).min(1.0);
        self.positions[0] += (mix.petrol_position - self.positions[0]) * smooth;
        self.positions[1] += (mix.motor_position - self.positions[1]) * smooth;
        let gains: Vec<f32> = bank_gains(self.positions[0], mix.petrol_gain).into_iter()
            .chain(bank_gains(self.positions[1], mix.motor_gain)).collect();
        for (i, gain) in gains.into_iter().enumerate() {
            if gain > 0.003 {
                if !self.active[i] {
                    play_sound(&self.sounds[i], PlaySoundParams { looped: true, volume: gain });
                    self.active[i] = true;
                } else { set_sound_volume(&self.sounds[i], gain); }
            } else if self.active[i] { stop_sound(&self.sounds[i]); self.active[i] = false; }
        }
    }

    pub fn motor_start(&self) {
        macroquad::audio::play_sound(&self.sounds[8], macroquad::audio::PlaySoundParams { looped: false, volume: 0.28 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_added_car_has_a_sound_profile() {
        for (name, expected) in [("Audi Q3",Voice::Petrol),("Tesla Cybertruck",Voice::Electric),("BMW i8",Voice::Hybrid),
            ("McLaren 720S",Voice::Sport),("Lamborghini Urus",Voice::Petrol),("Porsche 911 C4S",Voice::Sport),("BMW 320i",Voice::Petrol),("KOS",Voice::Petrol)] {
            assert_eq!(Voice::for_name(name), expected);
        }
    }
    #[test]
    fn electric_and_hybrid_mixes_use_the_correct_banks() {
        let ev = driving_mix(Voice::Electric, 20.0, true);
        assert_eq!(ev.petrol_gain, 0.0);
        assert!(ev.motor_gain > driving_mix(Voice::Electric, 0.0, false).motor_gain);
        let hybrid = driving_mix(Voice::Hybrid, 20.0, true);
        assert!(hybrid.petrol_gain > 0.0 && hybrid.motor_gain > 0.0);
        assert!(driving_mix(Voice::Petrol, 10.0, true).petrol_position > driving_mix(Voice::Petrol, 11.0, true).petrol_position);
    }
    #[test]
    fn gain_crossfades_are_bounded_and_only_use_adjacent_loops() {
        for i in 0..=140 {
            let gains = bank_gains(i as f32 / 20.0, 0.7);
            assert!((gains.iter().sum::<f32>() - 0.7).abs() < 1e-5);
            assert!(gains.iter().all(|v| *v >= 0.0 && *v <= 0.7));
            assert!(gains.iter().filter(|v| **v > 0.0).count() <= 2);
        }
        assert_eq!(bank_gains(3.5, 0.0), [0.0;8]);
        assert!(driving_mix(Voice::Sport, f32::NAN, true).petrol_position.is_finite());
    }
}
