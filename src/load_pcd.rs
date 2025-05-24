use anyhow::Result;
use pcd_rs::{PcdDeserialize, Reader};

use core::f32;
use std::{fs, path::Path, thread::sleep, time::Duration};

#[derive(Debug, PcdDeserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn load_pcd(path: &str) -> Result<Vec<Point>, Box<dyn std::error::Error>> {
    let reader = match Reader::open(path) {
        Ok(reader) => reader,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            return Err(e.into());
        }
    };

    let points: Vec<Point> = match reader.collect() {
        Ok(points) => points,
        Err(e) => {
            eprintln!("Error reading points: {}", e);
            return Err(e.into());
        }
    };
    // println!("There are {} points", points.len());

    Ok(points)
}

pub fn load_pcd_paths(dir_path: &str, data_type: &str) -> Result<Vec<String>, anyhow::Error> {
    let mut paths: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(dir_path) {
        println!("Reading directory: {}", dir_path);
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                // println!("Data: {}", path.to_str().unwrap());

                if let Some(ext) = path.extension() {
                    if ext == "pcd" {
                        if let Some(path_str) = path.to_str() {
                            paths.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Error reading directory: {}", dir_path);
        return Err(anyhow::anyhow!("Failed to read directory").into());
    }

    paths.sort_by_key(|path| {
        let filename = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        filename
            .strip_prefix(&format!("{}_", data_type))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    });
    // println!("Sorted paths: {:?}", paths);

    Ok(paths)
}
