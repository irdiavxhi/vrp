use super::*;
use crate::helpers::solver::population::*;
use crate::solver::population::non_dominated_sort::non_dominated_sort;

#[test]
fn can_get_crowding_distance() {
    // construct a multi objective over a Tuple
    let mo = TupleMultiObjective::new(vec![Box::new(Objective1), Box::new(Objective2)]);

    let a = Tuple(1, 3);
    let b = Tuple(3, 1);
    let c = Tuple(3, 3);
    let d = Tuple(2, 2);

    let solutions = vec![a, b, c, d];

    let f0 = non_dominated_sort(&solutions, &mo);

    let solutions = f0.iter().collect::<Vec<_>>();
    assert_eq!(3, solutions.len());
    assert_eq!(&a, solutions[0].0);
    assert_eq!(&b, solutions[1].0);
    assert_eq!(&d, solutions[2].0);

    let (crowding, stat) = assign_crowding_distance(&f0, &mo);

    assert_eq!(2, stat.len());
    assert_eq!(2.0, stat[0].spread);
    assert_eq!(2.0, stat[1].spread);

    // Same number as solutions in front
    assert_eq!(3, crowding.len());
    // All have rank 0
    assert_eq!(0, crowding[0].rank);
    assert_eq!(0, crowding[1].rank);
    assert_eq!(0, crowding[2].rank);

    let ca = crowding.iter().find(|i| i.solution.eq(&a)).unwrap();
    let cb = crowding.iter().find(|i| i.solution.eq(&b)).unwrap();
    let cd = crowding.iter().find(|i| i.solution.eq(&d)).unwrap();

    assert_eq!(INFINITY, ca.crowding_distance);
    assert_eq!(INFINITY, cb.crowding_distance);

    // only cd is in the middle. spread is in both dimensions the same
    // (2.0). norm is 1.0 / (spread * #objectives) = 1.0 / 4.0. As we
    // add two times 0.5, the crowding distance should be 1.0.
    assert_eq!(1.0, cd.crowding_distance);

    let f1 = f0.next_front();
    let solutions = f1.iter().collect::<Vec<_>>();
    assert_eq!(1, solutions.len());
    assert_eq!(&c, solutions[0].0);

    assert_eq!(true, f1.next_front().is_empty());
}
