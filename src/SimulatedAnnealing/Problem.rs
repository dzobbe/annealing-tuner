//////////////////////////////////////////////////////////////////////////////
//  File: neil/problem.rs
//////////////////////////////////////////////////////////////////////////////
//  Copyright 2016 Samuel Sleight
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//////////////////////////////////////////////////////////////////////////////

use super::super::Parameters;
use super::super::ThreadExecutor;
use std::collections::HashMap;
/**
 * A problem represents something to be solved using simulated
 * annealing, and provides methods to calculate the energy of a
 * state and generate new states.
 */
pub trait Problem {
    type State;

    /**
     * This function should generate an initial state for the problem.
     */
    fn initial_state(&mut self) -> Self::State;

    /**
     * This function should calculate the energy of a given state,
     * as a number between 0.0 and 1.0.
     *
     * Lower energy means the state is more optimal - simulated
     * annealing will try to find a state with the lowest energy.
     */
    fn energy(& mut self, state: &Self::State) -> f64;

    /**
     * This function should provide a new state, given the previous
     * state.
     */
    fn new_state(& mut self, state: &Self::State, max_steps: u64, current_step: u64) -> Self::State;
}

pub struct ProblemInputs{
	pub params_configurator:  Parameters::ParamsConfigurator,
	pub thread_executor:		  ThreadExecutor::ThreadExecutor,
}


impl Problem for ProblemInputs{
	type State= HashMap<String,u32>;
	
	fn initial_state(&mut self) -> Self::State{
		println!("Started Extraction of Initial State: it takes the Parameters Configuration given in input");
		let mut param_conf=self.params_configurator.get_initial_param_conf();
		return param_conf;
	}

  	fn energy(& mut self, state: &Self::State) -> f64{
  		println!("Started Energy Evaluation: it starts the execution of the benchmark for the specific parameter configuration and evaluate the performance result");
    	let mut perf_result=self.thread_executor.execute_test_instance(state);
    	return perf_result;
    }

    fn new_state(& mut self, state: &Self::State, max_steps: u64, current_step: u64) -> Self::State {
    	println!("Started Extraction of New State from Neighborhood Set");
		match self.params_configurator.get_neighborhood_params(max_steps, current_step) {
    		//There is a neighborhood available to return
    		Some(x) => return x,
    		//No neighborhood available, return a random state
   		 	None    => return self.params_configurator.get_random_state(),
		};
			
    }
}