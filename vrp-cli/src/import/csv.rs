#[cfg(test)]
#[path = "../../tests/unit/import/csv_test.rs"]
mod csv_test;

extern crate serde;
use serde::Deserialize;

extern crate csv;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{BufReader, Read};
use std::ops::Deref;
use vrp_pragmatic::json::problem::*;
use vrp_pragmatic::json::Location;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct CsvJob {
    id: String,
    lat: f64,
    lng: f64,
    demand: i32,
    duration: usize,
    tw_start: Option<String>,
    tw_end: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct CsvVehicle {
    id: String,
    lat: f64,
    lng: f64,
    capacity: i32,
    tw_start: String,
    tw_end: String,
    amount: usize,
    profile: String,
}

fn read_csv_entries<T, R: Read>(reader: BufReader<R>) -> Result<Vec<T>, Box<dyn Error>>
where
    for<'de> T: Deserialize<'de>,
{
    let mut reader = csv::Reader::from_reader(reader);
    let mut entries = vec![];

    for entry in reader.deserialize() {
        entries.push(entry?);
    }

    Ok(entries)
}

fn parse_tw(start: Option<String>, end: Option<String>) -> Option<Vec<String>> {
    match (start, end) {
        (Some(start), Some(end)) => Some(vec![start, end]),
        _ => None,
    }
}

fn read_jobs<R: Read>(reader: BufReader<R>) -> Result<Vec<Job>, Box<dyn Error>> {
    let get_task = |job: &CsvJob| JobTask {
        places: vec![JobPlace {
            location: Location { lat: job.lat, lng: job.lng },
            duration: job.duration as f64 * 60.,
            times: parse_tw(job.tw_start.clone(), job.tw_end.clone()).map(|tw| vec![tw]),
        }],
        demand: if job.demand != 0 { Some(vec![job.demand.abs()]) } else { None },
        tag: None,
    };

    let get_tasks = |jobs: &Vec<&CsvJob>, filter: Box<dyn Fn(&CsvJob) -> bool>| {
        let tasks = jobs.iter().filter(|j| filter.deref()(j)).map(|job| get_task(job)).collect::<Vec<_>>();
        if tasks.is_empty() {
            None
        } else {
            Some(tasks)
        }
    };

    let jobs = read_csv_entries::<CsvJob, _>(reader)?
        .iter()
        .fold(HashMap::new(), |mut acc, job| {
            acc.entry(&job.id).or_insert_with(|| vec![]).push(job);
            acc
        })
        .into_iter()
        .map(|(job_id, tasks)| Job {
            id: job_id.clone(),
            pickups: get_tasks(&tasks, Box::new(|j| j.demand > 0)),
            deliveries: get_tasks(&tasks, Box::new(|j| j.demand < 0)),
            replacements: None,
            services: get_tasks(&tasks, Box::new(|j| j.demand == 0)),
            priority: None,
            skills: None,
        })
        .collect();

    Ok(jobs)
}

fn read_vehicles<R: Read>(reader: BufReader<R>) -> Result<Vec<VehicleType>, Box<dyn Error>> {
    let vehicles = read_csv_entries::<CsvVehicle, _>(reader)?
        .into_iter()
        .map(|vehicle| {
            let depot_location = Location { lat: vehicle.lat, lng: vehicle.lng };

            VehicleType {
                type_id: vehicle.id.clone(),
                vehicle_ids: (1..vehicle.amount).map(|seq| format!("{}_{}", vehicle.profile, seq)).collect(),
                profile: vehicle.profile,
                costs: VehicleCosts { fixed: Some(25.), distance: 0.0002, time: 0.005 },
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: vehicle.tw_start, location: depot_location.clone() },
                    end: Some(VehiclePlace { time: vehicle.tw_end, location: depot_location }),
                    breaks: None,
                    reloads: None,
                }],
                capacity: vec![vehicle.capacity],
                skills: None,
                limits: None,
            }
        })
        .collect();

    Ok(vehicles)
}

fn create_format_error(entity: &str, error: Box<dyn Error>) -> FormatError {
    FormatError::new_with_details(
        "E0000".to_string(),
        format!("cannot read {}", entity),
        format!("check {} definition", entity),
        format!("{}", error),
    )
}

/// Reads problem from csv format.
pub fn read_csv_problem<R1: Read, R2: Read>(
    jobs_reader: BufReader<R1>,
    vehicles_reader: BufReader<R2>,
) -> Result<Problem, FormatError> {
    let jobs = read_jobs(jobs_reader).map_err(|err| create_format_error("jobs", err))?;
    let vehicles = read_vehicles(vehicles_reader).map_err(|err| create_format_error("vehicles", err))?;
    let profiles = vehicles.iter().map(|v| v.profile.clone()).collect::<HashSet<_>>();

    Ok(Problem {
        plan: Plan { jobs, relations: None },
        fleet: Fleet {
            vehicles,
            profiles: profiles.into_iter().map(|p| Profile { name: p.clone(), profile_type: p }).collect(),
        },
        objectives: None,
        config: None,
    })
}