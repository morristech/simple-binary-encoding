
/// Note that the included codec file and the data file further below
/// are generated through SBE's gradle tooling. `./gradlew generateRustCodecs`
include!("car_example_generated_codec.rs");

use std::fs::File;
use std::io::prelude::*;
use std::io::Write;

pub fn main() {
    ::std::process::exit(match run_car_example() {
                             Ok(_) => 0,
                             Err(e) => {
                                 writeln!(::std::io::stderr(), "error: {:?}", e).unwrap();
                                 1
                             }
                         });
}

fn read_sbe_file_generated_from_java_example() -> IoResult<Vec<u8>> {
    // Generated by the generateCarExampleDataFile gradle task.
    let mut f = File::open("car_example_data.sbe")?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn run_car_example() -> IoResult<()> {
    let reference_example_bytes = read_sbe_file_generated_from_java_example()?;
    decode_car_and_assert_expected_content(&reference_example_bytes)?;
    let bytes_encoded_from_rust = encode_car_from_scratch()?;
    assert_eq!(reference_example_bytes, bytes_encoded_from_rust);
    decode_car_and_assert_expected_content(&bytes_encoded_from_rust)?;
    Ok(())
}

fn decode_car_and_assert_expected_content(buffer: &[u8]) -> IoResult<()> {
    let (h, dec_fields) = start_decoding_car(&buffer).header()?;
    assert_eq!(49u16, h.block_length);
    assert_eq!(1u16, h.template_id);
    assert_eq!(1u16, h.schema_id);
    assert_eq!(0u16, h.version);
    println!("Header read");


    let mut found_fuel_figures = Vec::<FuelFigure>::with_capacity(EXPECTED_FUEL_FIGURES.len());

    let (fields, dec_fuel_figures_header) = dec_fields.car_fields()?;
    assert_eq!(1234, fields.serial_number);
    assert_eq!(2013, fields.model_year);
    assert_eq!(BooleanType::T, fields.available);
    assert_eq!([97_i8, 98, 99, 100, 101, 102], fields.vehicle_code); // abcdef
    assert_eq!([0_u32, 1, 2, 3, 4], fields.some_numbers);

    let dec_perf_figures_header = match dec_fuel_figures_header.fuel_figures_individually()? {
        Either::Left(mut dec_ff_members) => {
            println!("Got some fuel figure members");
            let mut decoder_after_group = None;
            loop {
                let (ff_fields, dec_usage_description) = dec_ff_members.next_fuel_figures_member()?;
                let (usage_description, next_step) = dec_usage_description.usage_description()?;
                let usage_str = std::str::from_utf8(usage_description).unwrap();
                println!("Fuel Figure: Speed: {0}, MPG: {1}, Usage: {2}",
                         ff_fields.speed,
                         ff_fields.mpg,
                         usage_str);
                found_fuel_figures.push(FuelFigure {
                                            speed: ff_fields.speed,
                                            mpg: ff_fields.mpg,
                                            usage_description: usage_str,
                                        });
                match next_step {
                    Either::Left(more_members) => dec_ff_members = more_members,
                    Either::Right(done_with_group) => {
                        decoder_after_group = Some(done_with_group);
                        break;
                    }
                }
            }
            decoder_after_group.unwrap()
        }
        Either::Right(next_decoder) => next_decoder,
    };
    assert!(EXPECTED_FUEL_FIGURES
                .iter()
                .zip(found_fuel_figures.iter())
                .all(|(exp, found)| exp == found),
            "fuel figures should match expected values");

    let dec_manufacturer = match dec_perf_figures_header.performance_figures_individually()? {
        Either::Left(mut dec_pf_members) => {
            let mut decoder_after_pf_group = None;
            println!("Got some performance figure members");
            loop {
                let (pf_fields, dec_acceleration_header) = dec_pf_members
                    .next_performance_figures_member()?;
                println!("Performance Figure Fixed Fields: Octane Rating: {0}",
                         pf_fields.octane_rating);
                let (accel_slice, next_step) = dec_acceleration_header.acceleration_as_slice()?;
                for accel_fields in accel_slice {
                    println!("Acceleration: MPH: {0}, Seconds: {1}",
                             accel_fields.mph,
                             accel_fields.seconds);
                }
                match next_step {
                    Either::Left(more_members) => dec_pf_members = more_members,
                    Either::Right(done_with_group) => {
                        decoder_after_pf_group = Some(done_with_group);
                        break;
                    }
                }
            }
            decoder_after_pf_group.unwrap()
        }
        Either::Right(next_decoder) => next_decoder,
    };
    let (manufacturer, dec_model) = dec_manufacturer.manufacturer()?;
    let manufacturer = std::str::from_utf8(manufacturer).unwrap();
    println!("Manufacturer: {}", manufacturer);
    assert_eq!("Honda", manufacturer);

    let (model, dec_activation_code) = dec_model.model()?;
    let model = std::str::from_utf8(model).unwrap();
    println!("Model: {}", model);
    assert_eq!("Civic VTi", model);

    let (activation_code, dec_done) = dec_activation_code.activation_code()?;
    let activation_code = std::str::from_utf8(activation_code).unwrap();
    println!("Activation Code: {}", activation_code);
    assert_eq!("abcdef", activation_code);

    let (position, buffer_back) = dec_done.unwrap();
    println!("Finished decoding. Made it to position {0} out of {1}",
             position,
             buffer_back.len());
    Ok(())
}

fn encode_car_from_scratch() -> IoResult<Vec<u8>> {
    let mut buffer = vec![0u8; 256];
    let used_pos = {
        let enc_header = start_encoding_car(&mut buffer);
        let enc_fields = enc_header
            .header_copy(&CarMessageHeader::default().message_header)?;
        println!("encoded header");
        let (fields, enc_fuel_figures_header) = enc_fields.car_fields()?;
        fields.serial_number = 1234;
        fields.model_year = 2013;
        fields.available = BooleanType::T;
        fields.code = Model::A;
        fields.vehicle_code = [97_i8, 98, 99, 100, 101, 102]; // abcdef
        fields.some_numbers = [0_u32, 1, 2, 3, 4];
        fields.extras = OptionalExtras(6);
        fields.engine = Engine {
            capacity: 2000,
            num_cylinders: 4,
            manufacturer_code: [49, 50, 51], // 123
            efficiency: 35,
            booster_enabled: BooleanType::T,
            booster: Booster {
                boost_type: BoostType::NITROUS,
                horse_power: 200,
            },
        };
        println!("encoded top level fields");
        let mut enc_fuel_figures = enc_fuel_figures_header.fuel_figures_individually()?;
        let mut fuel_figure_scratch = CarFuelFiguresMember { speed: 0, mpg: 0.0 };
        for ff in EXPECTED_FUEL_FIGURES {
            fuel_figure_scratch.speed = ff.speed;
            fuel_figure_scratch.mpg = ff.mpg;
            //fuel_figure_scratch.usage_description = ff.mpg;
            let enc_usage = enc_fuel_figures
                .next_fuel_figures_member(&fuel_figure_scratch)?;
            enc_fuel_figures = enc_usage
                .usage_description(ff.usage_description.as_bytes())?;
        }
        let enc_perf_figures_header = enc_fuel_figures.done_with_fuel_figures()?;
        println!("encoded fuel figures");
        let mut perf_figure_member_scratch = CarPerformanceFiguresMember { octane_rating: 0 };
        let mut enc_perf_figures = enc_perf_figures_header.performance_figures_individually()?;
        for pf in EXPECTED_PERF_FIXTURES {
            perf_figure_member_scratch.octane_rating = pf.octane_rating;
            let enc_accel = enc_perf_figures
                .next_performance_figures_member(&perf_figure_member_scratch)?;
            enc_perf_figures = enc_accel.acceleration_from_slice(&pf.acceleration)?;
        }
        let enc_manufacturer = enc_perf_figures.done_with_performance_figures()?;
        println!("encoded perf figures");
        let enc_model = enc_manufacturer.manufacturer("Honda".as_bytes())?;
        let enc_activation_code = enc_model.model("Civic VTi".as_bytes())?;
        let done = enc_activation_code.activation_code("abcdef".as_bytes())?;
        let (pos, _) = done.unwrap();
        pos
    };
    println!("encoded up to position {}", used_pos);
    buffer.truncate(used_pos);

    Ok(buffer)
}

#[derive(Debug, PartialEq)]
struct FuelFigure<'d> {
    speed: u16,
    mpg: f32,
    usage_description: &'d str,
}

const EXPECTED_FUEL_FIGURES: &'static [FuelFigure] = &[FuelFigure {
     speed: 30,
     mpg: 35.9,
     usage_description: "Urban Cycle",
 },
 FuelFigure {
     speed: 55,
     mpg: 49.0,
     usage_description: "Combined Cycle",
 },
 FuelFigure {
     speed: 75,
     mpg: 40.0,
     usage_description: "Highway Cycle",
 }];

struct PerfFigure {
    octane_rating: u8,
    acceleration: [CarPerformanceFiguresAccelerationMember; 3],
}

const EXPECTED_PERF_FIXTURES: &'static [PerfFigure] = &[PerfFigure {
     octane_rating: 95,
     acceleration: [CarPerformanceFiguresAccelerationMember {
                        mph: 30,
                        seconds: 4.0,
                    },
                    CarPerformanceFiguresAccelerationMember {
                        mph: 60,
                        seconds: 7.5,
                    },
                    CarPerformanceFiguresAccelerationMember {
                        mph: 100,
                        seconds: 12.2,
                    }],
 },
 PerfFigure {
     octane_rating: 99,
     acceleration: [CarPerformanceFiguresAccelerationMember {
                        mph: 30,
                        seconds: 3.8,
                    },
                    CarPerformanceFiguresAccelerationMember {
                        mph: 60,
                        seconds: 7.1,
                    },
                    CarPerformanceFiguresAccelerationMember {
                        mph: 100,
                        seconds: 11.8,
                    }],
 }];