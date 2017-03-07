
extern crate rustc_serialize;
extern crate rand;
extern crate libc;
extern crate time;
extern crate ansi_term;
extern crate pbr;
extern crate csv;
extern crate hwloc;
extern crate num_cpus;
extern crate wait_timeout;
extern crate ssh2;
extern crate xml;
extern crate ctrlc;

#[macro_use]
extern crate futures;
#[macro_use]
extern crate tokio_core;
extern crate futures_cpupool;

#[macro_use]
extern crate lazy_static;



use ansi_term::Colour::{Green, Yellow};
use std::collections::HashMap;
use annealing::problem::Problem;
use annealing::solver::seqsa::Seqsa;
use annealing::solver::Solver;
use annealing::solver::common::MrResult;

mod states_gen;
mod annealing;
mod energy_eval;
mod results_emitter;
mod xml_reader;
mod shared;

type State = HashMap<String, usize>;



/**
Annealing Tuner Entry Point
**/
fn main() {



  	/**
	Define the reader of the configuration xml file
	**/
    let xml_reader = xml_reader::XMLReader::new("conf.xml".to_string());


    /// Create ParamsConfigurator useful to manage the parameters (or states)
    /// that the simulated annealing algorithm will explore. It needs the xml_reader to 
    /// see 
    
    let params_config = states_gen::ParamsConfigurator::new(xml_reader.clone());




    /// Instantiate the EnergyEval struct needed for start/stop the Target and the Benchmark applications
    /// and then evaluate the energy selected by the user
    ///
    let energy_eval = energy_eval::EnergyEval { xml_reader: xml_reader.clone() };



    /// Configure the Simulated Annealing problem with the ParamsConfigurator and EnergyEval instances.
    /// Finally,the solver is started
    ///
    let mut problem = Problem {
        problem_type: xml_reader.ann_problem(),
        params_configurator: params_config,
        energy_evaluator: energy_eval,
    };



    let (t_min, t_max) = eval_temperature(xml_reader.ann_min_temp(),
                                          xml_reader.ann_max_temp(),
                                          &mut problem);


    let mr_result = match xml_reader.ann_version() {
        SolverVersion::seqsa => {
            let mut solver = annealing::solver::seqsa::Seqsa {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: xml_reader.ann_max_steps(),
                energy_type: xml_reader.ann_energy(),
                cooling_schedule: xml_reader.ann_cooling(),
            };

            solver.solve(&mut problem,1)
        }
        SolverVersion::spisa => {
            let mut solver = annealing::solver::spisa::Spisa {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: xml_reader.ann_max_steps(),
                energy_type: xml_reader.ann_energy(),
                cooling_schedule: xml_reader.ann_cooling(),
            };

            solver.solve(&mut problem, xml_reader.ann_workers())
        }
        SolverVersion::mir => {
            let mut solver = annealing::solver::mir::Mir {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: xml_reader.ann_max_steps(),
                energy_type: xml_reader.ann_energy(),
                cooling_schedule: xml_reader.ann_cooling(),
            };

            solver.solve(&mut problem, xml_reader.ann_workers())
        }
        SolverVersion::prsa => {
            let mut solver = annealing::solver::prsa::Prsa {
                min_temp: t_min,
                max_temp: t_max,
                max_steps: xml_reader.ann_max_steps(),
                population_size: 32,
                energy_type: xml_reader.ann_energy(),
                cooling_schedule: xml_reader.ann_cooling(),
            };

            solver.solve(&mut problem, xml_reader.ann_workers())
        }
    };

    println!("{}",Yellow.paint("\n-----------------------------------------------------------------------------------------------------------------------------------------------"));
    println!("{} {:?}",
             Yellow.paint("The Best Configuration found is: "),
             mr_result.state);
    println!("{} {:?}", Yellow.paint("Energy: "), mr_result.energy);
    println!("{}",Yellow.paint("-----------------------------------------------------------------------------------------------------------------------------------------------"));


}


/// Check if the temperature is given by the user or if Tmin and Tmax need to be evaluated
fn eval_temperature(t_min: Option<f64>, t_max: Option<f64>, problem: &mut Problem) -> (f64, f64) {
    let num_exec = 10;

    let min_temp = match t_min {
        Some(val) => val,
        None => 1.0,
    };

    let mut rng = rand::thread_rng();

    let max_temp = match t_max {
        Some(val) => val,
        None => {
            let mut deltas: Vec<f64> = Vec::with_capacity(num_exec);
            /// Search for Tmax: a temperature that gives 98% acceptance
            /// Tmin: equal to 1.
            println!("{} Temperature not provided. Starting its Evaluation",
                     Green.paint("[TUNER]"));
            let mut state = problem.initial_state();
            let mut energy=match problem.energy(&state, 0, rng.clone()) {
                Some(nrg) => nrg,
                None => panic!("The initial configuration does not allow to calculate the energy"),
            };

            for i in 0..num_exec {

                let next_state = problem.rand_state();
                let new_energy=match problem.energy(&next_state, 0, rng.clone()) {
                    Some(new_nrg) => deltas.push((energy-new_nrg).abs()),
                    None => {
                        println!("{} The current configuration parameters cannot be evaluated. \
                                  Skip!",
                                 Green.paint("[TUNER]"));
                    }
                };
                
            }

            let desired_prob: f64 = 0.98;
            let sum_deltas: f64=deltas.iter().cloned().sum();
            //(energies.iter().cloned().fold(0. / 0., f64::max) -energies.iter().cloned().fold(0. / 0., f64::min))
            (sum_deltas /deltas.len() as f64)/ (-desired_prob.ln())
        }
    };

    return (min_temp, max_temp);
}


#[derive(Debug, Clone,RustcDecodable)]
pub enum BenchmarkName {
    wrk,
    ycsb,
    memaslap,
}


#[derive(Debug, Clone,RustcDecodable)]
pub enum ProblemType {
    default,
    rastr,
    griew,
}

#[derive(Debug, Clone,RustcDecodable)]
pub enum CoolingSchedule {
    linear,
    exponential,
    basic_exp_cooling,
}

#[derive(Debug, Clone,RustcDecodable)]
pub enum SolverVersion {
    seqsa,
    spisa,
    mir,
    prsa,
}

#[derive(Debug, Clone, Copy, RustcDecodable)]
pub enum EnergyType {
    throughput,
    latency,
}


#[derive(Debug, Clone,RustcDecodable)]
pub enum ExecutionType {
    local,
    remote,
}

impl std::str::FromStr for BenchmarkName {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "wrk" => Ok(BenchmarkName::wrk),
            "ycsb" => Ok(BenchmarkName::ycsb),
            "memaslap" => Ok(BenchmarkName::memaslap),
            _ => Err("Benchmark Name - not a valid value"),
        }
    }
}

impl std::str::FromStr for ProblemType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "default" => Ok(ProblemType::default),
            "rastr" => Ok(ProblemType::rastr),
            "griew" => Ok(ProblemType::griew),
            _ => Err("Problem Type - not a valid value"),
        }
    }
}

impl std::str::FromStr for CoolingSchedule {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linear" => Ok(CoolingSchedule::linear),
            "exponential" => Ok(CoolingSchedule::exponential),
            "basic_exp_cooling" => Ok(CoolingSchedule::basic_exp_cooling),
            _ => Err("Cooling Schedule - not a valid value"),
        }
    }
}

impl std::str::FromStr for SolverVersion {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "seqsa" => Ok(SolverVersion::seqsa),
            "spisa" => Ok(SolverVersion::spisa),
            "mir" => Ok(SolverVersion::mir),
            "prsa" => Ok(SolverVersion::prsa),
            _ => Err("Solver Version - not a valid value"),
        }
    }
}

impl std::str::FromStr for EnergyType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "throughput" => Ok(EnergyType::throughput),
            "latency" => Ok(EnergyType::latency),
            _ => Err("Energy Type - not a valid value"),
        }
    }
}

impl std::str::FromStr for ExecutionType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(ExecutionType::local),
            "remote" => Ok(ExecutionType::remote),
            _ => Err("Execution Type - not a valid value"),
        }
    }
}
