use std::hint::black_box;
use std::time::Instant;

use typesafe_jev_sdk::Question;
use typesafe_jev_sdk::builder::JevRequestBuilder;

fn main() {
    let iterations = 200_000;
    for custom_model in [false, true] {
        let mut samples = Vec::new();
        for _ in 0..9 {
            let start = Instant::now();
            for _ in 0..iterations {
                let mut builder = JevRequestBuilder::new();
                if custom_model {
                    builder = builder.model(black_box("custom-model"));
                }
                let request = builder
                    .state(black_box("state"))
                    .question(
                        "ready",
                        Question::Noul {
                            instructions: "Ready?".to_owned(),
                            criteria: None,
                        },
                    )
                    .build()
                    .unwrap();
                black_box(request);
            }
            samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
        }
        samples.sort_by(f64::total_cmp);
        println!(
            "builder custom_model={custom_model}: median {:.1} ns/iteration (min {:.1}, max {:.1}; 9 samples x {iterations})",
            samples[4], samples[0], samples[8]
        );
    }
}
