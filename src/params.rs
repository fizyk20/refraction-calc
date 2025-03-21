use atm_refraction::{
    air::{us76_atmosphere, Atmosphere, AtmosphereDef},
    EarthShape, Environment, Path,
};
use clap::{App, AppSettings, Arg};
use std::{fs::File, io::Read};

/// Ray direction description
pub enum RayDir {
    /// angle from the horizon
    Angle(f64),
    /// hit a given altitude at the given distance
    Target { h: f64, dist: f64 },
    /// special value for finding the horizon
    Horizon,
}

/// Definition of a ray
pub struct RayData {
    /// starting altitude
    pub start_h: f64,
    /// direction of propagation
    pub dir: RayDir,
}

/// what info to output
pub enum Output {
    /// altitude at a given distance
    HAtDist(f64),
    /// output starting angle of the ray
    Angle,
    /// output the angle to the horizon
    HorizonAngle,
    /// Output the distance to the horizon
    HorizonDistance,
    /// Output the angle of deflection for rays from celestial objects
    Astronomical,
}

pub struct Params {
    pub ray: RayData,
    pub env: Environment,
    pub straight: bool,
    pub output: Vec<Output>,
    pub verbose: bool,
}

pub fn parse_arguments() -> Params {
    let matches = App::new("Atmospheric Refraction Calculator")
        .about("Calculates paths of light in Earth's atmosphere")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::with_name("start_h")
                .short("h")
                .long("start-h")
                .value_name("ALTITUDE")
                .help("Starting point altitude (meters) (default = 1)")
                .takes_value(true),
        ).arg(
            Arg::with_name("start_angle")
                .short("a")
                .long("start-angle")
                .value_name("ANGLE")
                .help("Starting direction, angle relative to horizontal (degrees); conflicts with --tgt-h and --tgt-dist")
                .takes_value(true),
        ).arg(
            Arg::with_name("target_h")
                .short("t")
                .long("tgt-h")
                .value_name("ALTITUDE")
                .help("Target point altitude (meters); conflicts with --start-angle")
                .takes_value(true),
        ).arg(
            Arg::with_name("target_dist")
                .short("d")
                .long("tgt-dist")
                .value_name("DISTANCE")
                .help("Target point distance (kilometers); conflicts with --start-angle")
                .takes_value(true),
        ).arg(
            Arg::with_name("radius")
                .short("R")
                .long("radius")
                .value_name("RADIUS")
                .help("Calculate assuming the given value as the Earth's radius, in km (default: 6378) (conflicts with --flat)")
                .takes_value(true),
        ).arg(
            Arg::with_name("flat")
                .short("f")
                .long("flat")
                .help("Simulate a flat Earth (conflicts with --radius)")
                .takes_value(false),
        ).arg(
            Arg::with_name("atmosphere")
                .long("atmosphere")
                .value_name("CONFIG")
                .help("Atmosphere configuration file")
                .takes_value(true),
        ).arg(
            Arg::with_name("output_dist")
                .short("o")
                .long("output-dist")
                .value_name("DISTANCE")
                .help("Distance at which to output altitude (kilometers)")
                .takes_value(true),
        ).arg(
            Arg::with_name("output_ang")
                .long("output-ang")
                .help("Output the starting angle of the ray")
                .takes_value(false),
        ).arg(
            Arg::with_name("output_horizon")
                .long("output-horizon")
                .help("Output the angle to the horizon")
                .takes_value(false),
        ).arg(
            Arg::with_name("output_horizon_dist")
                .long("output-horizon-dist")
                .help("Output the distance to the horizon")
                .takes_value(false),
        ).arg(
            Arg::with_name("output_astronomical")
                .long("output-astronomical")
                .help("Output the angle of deflection of rays from celestial objects")
                .takes_value(false),
        ).arg(
            Arg::with_name("straight")
                .short("s")
                .long("straight")
                .help("Calculation for a straight-line ray")
                .takes_value(false),
        ).arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("Be verbose")
                .takes_value(false),
        ).get_matches();
    let start_h: f64 = matches
        .value_of("start_h")
        .unwrap_or("1.0")
        .parse()
        .ok()
        .expect("Invalid altitude passed to start-h");
    let start_angle = matches.value_of("start_angle");
    let tgt_h = matches.value_of("target_h");
    let tgt_dist = matches.value_of("target_dist");

    let ray_dir =
        if matches.is_present("output_horizon") || matches.is_present("output_horizon_dist") {
            RayDir::Horizon
        } else {
            match (start_angle, tgt_h, tgt_dist) {
                (Some(ang), None, None) => RayDir::Angle(
                    ang.parse()
                        .ok()
                        .expect("Invalid angle passed to --start-angle"),
                ),
                (None, Some(h), Some(dist)) => RayDir::Target {
                    h: h.parse().ok().expect("Invalid altitude passed to --tgt-h"),
                    dist: dist
                        .parse::<f64>()
                        .ok()
                        .expect("Invalid distance passed to --tgt-dist")
                        * 1e3,
                },
                (None, None, None) => panic!("No ray direction chosen!"),
                _ => panic!("Conflicting options detected (--start-angle, --tgt-h, --tgt-dist)"),
            }
        };
    let ray = RayData {
        start_h,
        dir: ray_dir,
    };

    let shape = match (matches.is_present("flat"), matches.value_of("radius")) {
        (false, None) => EarthShape::Spherical { radius: 6378000.0 },
        (true, None) => EarthShape::Flat,
        (false, Some(radius)) => {
            let r: f64 = radius.parse().ok().expect("Invalid radius passed");
            EarthShape::Spherical { radius: r * 1e3 }
        }
        (true, Some(_)) => panic!("Conflicting Earth shape options chosen!"),
    };

    let atmosphere = matches
        .value_of("atmosphere")
        .map(|file| get_atmosphere(&file))
        .unwrap_or_else(us76_atmosphere);

    let mut output = Vec::new();
    if let Some(dist) = matches
        .value_of("output_dist")
        .and_then(|val| val.parse::<f64>().ok())
    {
        output.push(Output::HAtDist(dist * 1e3));
    }
    if matches.is_present("output_ang") {
        output.push(Output::Angle);
    }
    if matches.is_present("output_astronomical") {
        output.push(Output::Astronomical);
    }
    if matches.is_present("output_horizon") {
        output = vec![Output::HorizonAngle];
    }
    if matches.is_present("output_horizon_dist") {
        output = vec![Output::HorizonDistance];
    }
    Params {
        ray,
        straight: matches.is_present("straight"),
        env: Environment {
            shape,
            atmosphere,
            wavelength: 530e-9,
        },
        output,
        verbose: matches.is_present("verbose"),
    }
}

pub fn create_path<'a>(params: &'a Params) -> Box<dyn Path<'a> + 'a> {
    match params.ray.dir {
        RayDir::Angle(ang) => {
            params
                .env
                .cast_ray(params.ray.start_h, ang.to_radians(), params.straight)
        }
        RayDir::Target { h, dist } => {
            params
                .env
                .cast_ray_target(params.ray.start_h, h, dist, params.straight)
        }
        RayDir::Horizon => params.env.cast_ray(0.0, 0.0, params.straight),
    }
}

fn get_atmosphere(path: &str) -> Atmosphere {
    let mut config_file =
        File::open(path).unwrap_or_else(|_| panic!("couldn't open the config file {:?}", path));
    let mut contents = String::new();
    config_file
        .read_to_string(&mut contents)
        .unwrap_or_else(|_| panic!("failed reading from file {:?}", path));
    let def = serde_yaml::from_str::<AtmosphereDef>(&contents).expect("failed parsing config file");
    Atmosphere::from_def(def)
}
