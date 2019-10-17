/* Uses predefined values to control algorithm execution.
int distribution values:
1. route index in solution
2*. job index in selected route tour
3*. selected algorithm: 1: sequential algorithm(**)
4*. string removal index(-ies)
double distribution values:
1. string count
2*. string size(-s)
(*) - specific for each route.
(**) - calls more int and double distributions:
    int 5. split start
    dbl 3. alpha param
*/

use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::refinement::generate_matrix_routes;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::ObjectiveCost;
use crate::refinement::ruin::{AdjustedStringRemoval, RuinStrategy};
use crate::refinement::RefinementContext;
use std::sync::Arc;

parameterized_test! {can_ruin_solution_with_matrix_routes, (matrix, ints, reals, expected_ids), {
    can_ruin_solution_with_matrix_routes_impl(matrix, ints, reals, expected_ids);
}}

can_ruin_solution_with_matrix_routes! {
    case_01_sequential: ((10, 1), vec![0, 3, 1, 2], vec![1., 5.], vec!["c1", "c2", "c3", "c4", "c5"]),
    case_02_preserved: ((10, 1), vec![0, 2, 2, 1, 4], vec![1., 5., 0.5, 0.005], vec!["c0", "c1", "c2", "c5", "c6"]),
    case_03_preserved: ((10, 1), vec![0, 2, 2, 1, 4], vec![1., 5., 0.5, 0.5, 0.005], vec!["c0", "c1", "c2", "c6", "c7"]),
    case_04_preserved: ((10, 1), vec![0, 2, 2, 3, 4], vec![1., 5., 0.5, 0.5, 0.005], vec!["c2", "c6", "c7", "c8", "c9"]),
}

fn can_ruin_solution_with_matrix_routes_impl(
    matrix: (usize, usize),
    ints: Vec<i32>,
    reals: Vec<f64>,
    expected_ids: Vec<&str>,
) {
    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1);
    let refinement_ctx = RefinementContext {
        problem: Arc::new(problem),
        locked: Default::default(),
        population: vec![(Arc::new(solution), ObjectiveCost::new(0., 0.))],
        random: Arc::new(FakeRandom::new(ints, reals)),
        generation: 0,
    };

    let insertion_ctx = AdjustedStringRemoval::default().ruin_solution(&refinement_ctx).unwrap();

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}