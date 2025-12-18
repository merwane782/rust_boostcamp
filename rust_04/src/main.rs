use clap::Parser;
use crossterm::{
    cursor,
    style::{Color, Print, SetForegroundColor},
    terminal, ExecutableCommand,
};
use rand::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "hexpath")]
#[command(about = "Pathfinding on hexadecimal grid", long_about = None)]
struct Args {
    map_file: Option<String>,
    #[arg(short, long)]
    generate: Option<String>,
    #[arg(short, long)]
    output: Option<String>,
    #[arg(short, long)]
    visualize: bool,
    #[arg(short, long)]
    both: bool,
    #[arg(short, long)]
    animate: bool,
}

#[derive(Clone)]
struct Grid {
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

impl Grid {
    fn generate_random(width: usize, height: usize) -> Self {
        let mut rng = rand::rng();
        let cells: Vec<u8> = (0..width * height)
            .map(|_| rng.random_range(0..=255))
            .collect();

        Grid {
            width,
            height,
            cells,
        }
    }

    fn get(&self, x: usize, y: usize) -> Option<u8> {
        if x < self.width && y < self.height {
            Some(self.cells[y * self.width + x])
        } else {
            None
        }
    }

    fn index_to_coords(&self, index: usize) -> (usize, usize) {
        (index % self.width, index / self.width)
    }

    fn coords_to_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn neighbors(&self, index: usize) -> Vec<usize> {
        let (x, y) = self.index_to_coords(index);
        let mut neighbors = Vec::new();

        if y > 0 {
            neighbors.push(self.coords_to_index(x, y - 1));
        }
        if y < self.height - 1 {
            neighbors.push(self.coords_to_index(x, y + 1));
        }
        if x > 0 {
            neighbors.push(self.coords_to_index(x - 1, y));
        }
        if x < self.width - 1 {
            neighbors.push(self.coords_to_index(x + 1, y));
        }

        neighbors
    }

    fn save_to_file(&self, filename: &str) -> io::Result<()> {
        let mut content = String::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(value) = self.get(x, y) {
                    content.push_str(&format!("{:02X} ", value));
                }
            }
            content.push('\n');
        }

        fs::write(filename, content)
    }

    fn load_from_file(filename: &str) -> io::Result<Self> {
        let content = fs::read_to_string(filename)?;
        let lines: Vec<&str> = content.lines().collect();
        let height = lines.len();
        let mut width = 0;
        let mut cells = Vec::new();

        for line in lines {
            let values: Vec<&str> = line.split_whitespace().collect();
            if width == 0 {
                width = values.len();
            }

            for val in values {
                let byte = u8::from_str_radix(val, 16)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                cells.push(byte);
            }
        }

        Ok(Grid {
            width,
            height,
            cells,
        })
    }
}

#[derive(Eq, PartialEq)]
struct State {
    cost: usize,
    position: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn value_to_color(value: u8) -> Color {
    let t = value as f32 / 255.0;
    if t < 0.33 {
        let s = t / 0.33;
        Color::Rgb {
            r: 0,
            g: (255.0 * s) as u8,
            b: (255.0 * (1.0 - s)) as u8,
        }
    } else if t < 0.66 {
        let s = (t - 0.33) / 0.33;
        Color::Rgb {
            r: (255.0 * s) as u8,
            g: 255,
            b: 0,
        }
    } else {
        let s = (t - 0.66) / 0.34;
        Color::Rgb {
            r: 255,
            g: (255.0 * (1.0 - s)) as u8,
            b: 0,
        }
    }
}

fn visualize_grid(grid: &Grid, path: Option<&HashSet<usize>>, animate: bool) -> io::Result<()> {
    let mut stdout = io::stdout();

    if animate {
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        stdout.execute(cursor::MoveTo(0, 0))?;
    }

    for y in 0..grid.height {
        for x in 0..grid.width {
            if let Some(value) = grid.get(x, y) {
                let index = grid.coords_to_index(x, y);
                let is_path = path.is_some_and(|p| p.contains(&index));
                if is_path {
                    stdout.execute(SetForegroundColor(Color::White))?;
                    stdout.execute(Print(format!("{:02X} ", value)))?;
                } else {
                    let color = value_to_color(value);
                    stdout.execute(SetForegroundColor(color))?;
                    stdout.execute(Print(format!("{:02X} ", value)))?;
                }
            }
        }
        stdout.execute(Print("\n"))?;
    }

    stdout.execute(SetForegroundColor(Color::Reset))?;
    stdout.flush()?;

    if animate {
        thread::sleep(Duration::from_millis(50));
    }

    Ok(())
}

fn dijkstra_min_path(
    grid: &Grid,
    start: usize,
    end: usize,
    animate: bool,
) -> Option<(Vec<usize>, usize)> {
    let mut dist = vec![usize::MAX; grid.cells.len()];
    let mut prev = vec![None; grid.cells.len()];
    let mut heap = BinaryHeap::new();
    let mut visited = HashSet::new();

    dist[start] = 0;
    heap.push(State {
        cost: 0,
        position: start,
    });

    while let Some(State { cost, position }) = heap.pop() {
        if position == end {
            let mut path = Vec::new();
            let mut current = Some(end);

            while let Some(pos) = current {
                path.push(pos);
                current = prev[pos];
            }

            path.reverse();
            return Some((path, cost));
        }

        if visited.contains(&position) {
            continue;
        }

        visited.insert(position);

        if animate {
            let _ = visualize_grid(grid, Some(&visited), true);
        }

        for neighbor in grid.neighbors(position) {
            if visited.contains(&neighbor) {
                continue;
            }

            let new_cost = cost + grid.cells[neighbor] as usize;
            if new_cost < dist[neighbor] {
                dist[neighbor] = new_cost;
                prev[neighbor] = Some(position);
                heap.push(State {
                    cost: new_cost,
                    position: neighbor,
                });
            }
        }
    }

    None
}

fn greedy_max_path(grid: &Grid, start: usize, end: usize) -> Option<(Vec<usize>, usize)> {
    let mut path = vec![start];
    let mut visited = HashSet::new();
    let mut current = start;
    let mut total_cost = 0;

    visited.insert(start);

    while current != end {
        let neighbors = grid.neighbors(current);
        let mut best_neighbor = None;
        let mut best_cost = 0;

        for &neighbor in &neighbors {
            if !visited.contains(&neighbor) {
                let cost = grid.cells[neighbor] as usize;
                if cost > best_cost {
                    best_cost = cost;
                    best_neighbor = Some(neighbor);
                }
            }
        }

        if let Some(next) = best_neighbor {
            total_cost += best_cost;
            current = next;
            visited.insert(current);
            path.push(current);
        } else {
            let mut found = false;
            for &neighbor in &neighbors {
                if !visited.contains(&neighbor) {
                    current = neighbor;
                    visited.insert(current);
                    path.push(current);
                    total_cost += grid.cells[neighbor] as usize;
                    found = true;
                    break;
                }
            }
            if !found {
                return None;
            }
        }

        if path.len() > grid.cells.len() {
            return None;
        }
    }

    Some((path, total_cost))
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let grid = if let Some(gen_spec) = &args.generate {
        let parts: Vec<&str> = gen_spec.split('x').collect();
        if parts.len() != 2 {
            eprintln!("Invalid format. Use WxH (e.g., 10x10)");
            std::process::exit(1);
        }

        let width = parts[0].parse::<usize>().expect("Invalid width");
        let height = parts[1].parse::<usize>().expect("Invalid height");
        let grid = Grid::generate_random(width, height);

        if let Some(output_file) = &args.output {
            grid.save_to_file(output_file)?;
            println!("✓ Map saved to {}", output_file);
        }

        grid
    } else if let Some(map_file) = &args.map_file {
        Grid::load_from_file(map_file)?
    } else {
        eprintln!("Must specify --generate or provide a map file");
        std::process::exit(1);
    };

    println!("📊 Grid: {}x{}", grid.width, grid.height);

    if args.visualize && !args.animate {
        println!("\n🎨 Map visualization:");
        visualize_grid(&grid, None, false)?;
    }

    let start = 0;
    let end = grid.cells.len() - 1;

    if args.both || (!args.visualize && args.output.is_none()) {
        println!("\n🔍 Finding paths from top-left to bottom-right...\n");

        if let Some((min_path, min_cost)) = dijkstra_min_path(&grid, start, end, args.animate) {
            println!("✓ Minimum cost path found!");
            println!(" Cost: {}", min_cost);
            println!(" Length: {} steps", min_path.len());

            if args.visualize {
                println!("\n🎨 Minimum path visualization:");
                let path_set: HashSet<usize> = min_path.into_iter().collect();
                visualize_grid(&grid, Some(&path_set), false)?;
            }
        } else {
            println!("✗ No minimum path found");
        }

        if args.both {
            println!();

            if let Some((max_path, max_cost)) = greedy_max_path(&grid, start, end) {
                println!("✓ Maximum cost path found (greedy approximation)!");
                println!(" Cost: {}", max_cost);
                println!(" Length: {} steps", max_path.len());

                if args.visualize {
