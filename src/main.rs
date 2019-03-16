mod params;

use crate::params::*;
use atm_refraction::Path;

fn find_dist_for_h(ray: &Path, tgt_h: f64) -> f64 {
    let (mut min_dist, mut max_dist) = (0.0, 5000e3);

    while max_dist - min_dist > 0.00001 {
        let cur_dist = 0.5 * (min_dist + max_dist);
        let h = ray.h_at_dist(cur_dist);
        if h > tgt_h {
            max_dist = cur_dist;
        } else {
            min_dist = cur_dist;
        }
    }

    0.5 * (min_dist + max_dist)
}

fn main() {
    let params = parse_arguments();

    if params.verbose {
        println!("Ray parameters chosen:");
        match params.env.radius() {
            Some(radius) => println!("Earth: spherical with radius {} km", radius / 1e3),
            None => println!("Earth: flat"),
        }
        println!("Starting altitude: {} m ASL", params.ray.start_h);
    }

    let ray = create_path(&params);

    if params.straight && params.verbose {
        println!("Straight-line calculation chosen.");
    }
    if params.verbose {
        println!();
    }

    for output in &params.output {
        match *output {
            Output::HAtDist(dist) => {
                if params.verbose {
                    println!("Altitude at distance {} km: {}", dist, ray.h_at_dist(dist));
                } else {
                    println!("{}", ray.h_at_dist(dist));
                }
            }
            Output::Angle => {
                if params.verbose {
                    println!(
                        "Starting angle: {} degrees",
                        ray.angle_at_dist(0.0).to_degrees()
                    );
                } else {
                    println!("{}", ray.angle_at_dist(0.0).to_degrees());
                }
            }
            Output::Horizon => {
                let dist_to_target_h = find_dist_for_h(&*ray, params.ray.start_h);
                let ang = ray.angle_at_dist(dist_to_target_h);
                if params.verbose {
                    println!("Angle to the horizon: {} degrees", -ang.to_degrees());
                } else {
                    println!("{}", -ang.to_degrees());
                }
            }
        }
    }
}
