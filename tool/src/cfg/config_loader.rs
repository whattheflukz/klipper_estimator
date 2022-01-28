use serde::Deserialize;
use lib_klipper::planner::{FirmwareRetractionOptions, MoveChecker, Planner, PrinterLimits};
use std::error::Error;
use lib_klipper::glam::DVec3;

pub struct Loader{
    limits: PrinterLimits
}

impl Default for Loader{
    fn default() -> Loader{
        Loader{limits: PrinterLimits::default()}
    }
}

impl Loader{

    pub fn apply_override(&mut self, override_property: String, override_value: String){
        let c = match override_property.as_str() {
            "max_acceleration" | "max_accel" | "accel" => self.limits.set_max_acceleration(override_value.parse::<f64>().unwrap()),
            "max_accel_to_decel" | "accel_to_decel" | "a2d" => self.limits.set_max_accel_to_decel(override_value.parse::<f64>().unwrap()),
            "max_velocity" | "velocity" | "max_vel" | "vel" => self.limits.set_max_velocity(override_value.parse::<f64>().unwrap()),
            "square_corner_velocity" | "scv" | "SCV" | "sqv" | "SQV" => self.limits.set_square_corner_velocity(override_value.parse::<f64>().unwrap()),
            "instant_corner_velocity" | "icv" | "ICV" => self.limits.set_instant_corner_velocity(override_value.parse::<f64>().unwrap()),
            _ => {
                println!("bad user input : {} {}",override_property, override_value);
            }
         };
    }

    pub fn load_config(mut self, config_filename: Option<&String>, config_moonraker: Option<&String>, config_overrides: Option<Vec<String>>)-> Result<PrinterLimits, Box<dyn Error>> {
        // Load config file

        self.limits = if let Some(filename) = &config_filename {
            let src = std::fs::read_to_string(filename)?;
            let mut limits: PrinterLimits = deser_hjson::from_str(&src)?;

            // Do any fix-ups
            limits.set_square_corner_velocity(self.limits.square_corner_velocity);

            limits
        } else {
            PrinterLimits::default()
        };

        // Was moonraker config requested? If so, try to grab that first.
        if let Some(url) = &config_moonraker {
            self.moonraker_config(url)?;
        }

        //check for overrides and apply them 
        match config_overrides{
            None => {}
            Some(i) => {
                for cfg_override in i {
                    let property: String = cfg_override.get(0..cfg_override.find(':').unwrap()).unwrap().to_string(); //this is cursed i gotta make this better
                    let value: String = cfg_override.get(cfg_override.find(':').unwrap()+1..).unwrap().to_string();
                    self.apply_override(property, value);
                }
            }
        }

        Ok(self.limits)
    }

    fn moonraker_config(&mut self, source_url: &str) -> Result<(), Box<dyn Error>> {
        let mut url = source_url.to_string();
        url.push_str("/printer/objects/query?configfile=settings");

        #[derive(Debug, Deserialize)]
        struct MoonrakerResultRoot {
            result: MoonrakerResult,
        }

        #[derive(Debug, Deserialize)]
        struct MoonrakerResult {
            status: MoonrakerResultStatus,
        }

        #[derive(Debug, Deserialize)]
        struct MoonrakerResultStatus {
            configfile: MoonrakerConfigFile,
        }

        #[derive(Debug, Deserialize)]
        struct MoonrakerConfigFile {
            settings: MoonrakerConfig,
        }

        #[derive(Debug, Deserialize)]
        struct MoonrakerConfig {
            printer: PrinterConfig,
            extruder: ExtruderConfig,
            firmware_retraction: Option<FirmwareRetractionConfig>,
        }

        #[derive(Debug, Deserialize)]
        struct PrinterConfig {
            max_velocity: f64,
            max_accel: f64,
            max_accel_to_decel: f64,
            square_corner_velocity: f64,

            max_x_velocity: Option<f64>,
            max_x_accel: Option<f64>,
            max_y_velocity: Option<f64>,
            max_y_accel: Option<f64>,
            max_z_velocity: Option<f64>,
            max_z_accel: Option<f64>,
        }

        #[derive(Debug, Deserialize)]
        struct ExtruderConfig {
            max_extrude_only_velocity: f64,
            max_extrude_only_accel: f64,
            instantaneous_corner_velocity: f64,
        }

        #[derive(Debug, Deserialize)]
        struct FirmwareRetractionConfig {
            retract_length: f64,
            unretract_extra_length: f64,
            unretract_speed: f64,
            retract_speed: f64,
            #[serde(default)]
            lift_z: f64,
        }

        let cfg = reqwest::blocking::get(url)?
            .json::<MoonrakerResultRoot>()?
            .result
            .status
            .configfile
            .settings;

        self.limits.set_max_velocity(cfg.printer.max_velocity);
        self.limits.set_max_acceleration(cfg.printer.max_accel);
        self.limits.set_max_accel_to_decel(cfg.printer.max_accel_to_decel);
        self.limits.set_square_corner_velocity(cfg.printer.square_corner_velocity);
        self.limits.set_instant_corner_velocity(cfg.extruder.instantaneous_corner_velocity);

        self.limits.firmware_retraction = cfg.firmware_retraction.map(|fr| FirmwareRetractionOptions {
            retract_length: fr.retract_length,
            unretract_extra_length: fr.unretract_extra_length,
            unretract_speed: fr.unretract_speed,
            retract_speed: fr.retract_speed,
            lift_z: fr.lift_z,
        });

        let limits = [
            (
                DVec3::X,
                cfg.printer.max_x_velocity,
                cfg.printer.max_x_accel,
            ),
            (
                DVec3::Y,
                cfg.printer.max_y_velocity,
                cfg.printer.max_y_accel,
            ),
            (
                DVec3::Z,
                cfg.printer.max_z_velocity,
                cfg.printer.max_z_accel,
            ),
        ];

        for (axis, m, a) in limits {
            if let (Some(max_velocity), Some(max_accel)) = (m, a) {
                self.limits.move_checkers.push(MoveChecker::AxisLimiter {
                    axis,
                    max_velocity,
                    max_accel,
                });
            }
        }

        self.limits.move_checkers.push(MoveChecker::ExtruderLimiter {
            max_velocity: cfg.extruder.max_extrude_only_velocity,
            max_accel: cfg.extruder.max_extrude_only_accel,
        });
        Ok(())
    }
}