use std::cmp::Ordering::Less;
use std::ops::{Add, Deref, Sub};
use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::construction::states::{InsertionContext, RouteContext, SolutionContext};
use vrp_core::models::problem::{Costs, Job};
use vrp_core::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCostType};
use vrp_core::refinement::RefinementContext;
use vrp_core::utils::get_stdev;

pub struct WorkBalance {}

impl WorkBalance {
    /// Creates `WorkBalanceModule` which balances max load across all tours.
    pub fn new_load_balanced<Capacity>(
        load_func: Arc<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    ) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>)
    where
        Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static,
    {
        let create_balance = || MaxLoadBalance::<Capacity> {
            load_func: load_func.clone(),
            default_capacity: Capacity::default(),
            default_intervals: vec![(0_usize, 0_usize)],
        };

        (
            Box::new(WorkBalanceModule {
                constraints: vec![ConstraintVariant::SoftRoute(Arc::new(create_balance()))],
                keys: vec![],
            }),
            Box::new(create_balance()),
        )
    }

    /// Creates `WorkBalanceModule` which balances activities across all tours.
    pub fn new_activity_balanced() -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn Objective + Send + Sync>) {
        (
            Box::new(WorkBalanceModule {
                constraints: vec![ConstraintVariant::SoftRoute(Arc::new(ActivityBalance {}))],
                keys: vec![],
            }),
            Box::new(ActivityBalance {}),
        )
    }
}

/// A module which provides way to balance work across all tours.
struct WorkBalanceModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl ConstraintModule for WorkBalanceModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct MaxLoadBalance<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    load_func: Arc<dyn Fn(&Capacity, &Capacity) -> f64 + Send + Sync>,
    default_capacity: Capacity,
    default_intervals: Vec<(usize, usize)>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    MaxLoadBalance<Capacity>
{
    fn get_max_load_ratio(&self, ctx: &RouteContext) -> f64 {
        let capacity = ctx.route.actor.vehicle.dimens.get_capacity().unwrap();

        let intervals =
            ctx.state.get_route_state::<Vec<(usize, usize)>>(RELOAD_INTERVALS).unwrap_or(&self.default_intervals);

        intervals
            .iter()
            .map(|(start, _)| ctx.route.tour.get(*start).unwrap())
            .map(|activity| {
                ctx.state
                    .get_activity_state::<Capacity>(MAX_FUTURE_CAPACITY_KEY, activity)
                    .unwrap_or_else(|| &self.default_capacity)
            })
            .map(|max_load| self.load_func.deref()(max_load, capacity))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
            .unwrap_or(0_f64)
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    SoftRouteConstraint for MaxLoadBalance<Capacity>
{
    fn estimate_job(&self, solution_ctx: &SolutionContext, route_ctx: &RouteContext, _job: &Job) -> f64 {
        let max_load_ratio = self.get_max_load_ratio(route_ctx);
        let max_cost = get_max_cost(solution_ctx);

        max_cost * max_load_ratio
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static> Objective
    for MaxLoadBalance<Capacity>
{
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        let max_loads = insertion_ctx.solution.routes.iter().map(|rc| self.get_max_load_ratio(rc)).collect();

        Box::new(MeasurableObjectiveCost::new(get_stdev(&max_loads)))
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, _: &InsertionContext) -> Option<bool> {
        None
    }
}

struct ActivityBalance {}

impl SoftRouteConstraint for ActivityBalance {
    fn estimate_job(&self, solution_ctx: &SolutionContext, route_ctx: &RouteContext, _job: &Job) -> f64 {
        let max_cost = get_max_cost(solution_ctx);

        route_ctx.route.tour.activity_count() as f64 * max_cost
    }
}

impl Objective for ActivityBalance {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        let activities = insertion_ctx.solution.routes.iter().map(|rc| rc.route.tour.activity_count() as f64).collect();

        Box::new(MeasurableObjectiveCost::new(get_stdev(&activities)))
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, _: &InsertionContext) -> Option<bool> {
        None
    }
}

fn get_max_cost(solution_ctx: &SolutionContext) -> f64 {
    let get_total_cost = |costs: &Costs, distance: f64, duration: f64| {
        costs.fixed
            + costs.per_distance * distance
            + costs.per_driving_time.max(costs.per_service_time).max(costs.per_waiting_time) * duration
    };

    solution_ctx
        .routes
        .iter()
        .map(|rc| {
            let distance = rc.state.get_route_state::<f64>(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
            let duration = rc.state.get_route_state::<f64>(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);

            get_total_cost(&rc.route.actor.vehicle.costs, distance, duration)
                + get_total_cost(&rc.route.actor.driver.costs, distance, duration)
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(0.)
}