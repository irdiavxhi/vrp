use crate::construction::states::*;
use crate::models::problem::Job;
use crate::models::solution::Route;
use crate::models::Solution;
use crate::refinement::RefinementContext;
use crate::utils::DefaultRandom;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Specifies ruin strategy.
pub trait RuinStrategy {
    fn ruin_solution(&self, ctx: &RefinementContext) -> Result<InsertionContext, String>;
}

/// Creates insertion context from existing solution.
fn create_insertion_context(refinement_ctx: &RefinementContext) -> Result<InsertionContext, String> {
    let (solution, cost) = refinement_ctx.population.first().ok_or("Empty population")?;
    let jobs: Vec<Arc<Job>> = solution.unassigned.iter().map(|(job, _)| job.clone()).collect();
    let mut registry = solution.registry.deep_copy();
    let mut routes: HashSet<RouteContext> = HashSet::new();

    solution.routes.iter().for_each(|route| {
        if route.tour.has_jobs() {
            let mut route_ctx = RouteContext {
                route: Arc::new(RwLock::new(route.deep_copy())),
                state: Arc::new(RwLock::new(RouteState::new())),
            };
            refinement_ctx.problem.constraint.accept_route_state(&mut route_ctx);
            routes.insert(route_ctx);
        } else {
            registry.free_actor(&route.actor);
        }
    });

    Ok(InsertionContext {
        progress: InsertionProgress {
            cost: Some(cost.total()),
            completeness: 1. - (solution.unassigned.len() as f64 / refinement_ctx.problem.jobs.size() as f64),
            total: refinement_ctx.problem.jobs.size(),
        },
        problem: refinement_ctx.problem.clone(),
        solution: SolutionContext { required: jobs, ignored: vec![], unassigned: Default::default(), routes, registry },
        random: refinement_ctx.random.clone(),
    })
}

mod adjusted_string_removal;

pub use self::adjusted_string_removal::AdjustedStringRemoval;