use clap::Parser;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// Find min/max cost paths in hexadecimal grid
#[derive(Parser, Debug)]
#[command(name = "hexpath", about, long_about = None, disable_version_flag = true)]
struct Args {
    /// Map file (hex values, space separated)
    map_file: Option<String>,

    /// Generate random map (e.g., 8x4, 10x10)
    #[arg(long)]
    generate: Option<String>,

    /// Save generated map to file
    #[arg(long)]
    output: Option<String>,

    /// Show colored map
    #[arg(long)]
    visualize: bool,

    /// Show both min and max paths
    #[arg(long)]
    both: bool,

    /// Animate pathfinding
    #[arg(long)]
    animate: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct State {
    cost: u32,
    pos: (usize, usize),
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

#[derive(Clone, Copy, PartialEq, Eq)]
struct StateMax {
    cost: u32,
    pos: (usize, usize),
}

impl Ord for StateMax {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cost.cmp(&other.cost)
    }
}

impl PartialOrd for StateMax {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_map(content: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut grid = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: Result<Vec<u8>, _> = line
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16))
            .collect();
        grid.push(row.map_err(|e| format!("Invalid hex value: {}", e))?);
    }

    if grid.is_empty() {
        return Err("Empty map".to_string());
    }

    let width = grid[0].len();
    for row in &grid {
        if row.len() != width {
            return Err("Inconsistent row lengths".to_string());
        }
    }

    Ok(grid)
}

fn generate_map(size_str: String) -> Result<Vec<Vec<u8>>, String> {
    let parts: Vec<&str> = size_str.split('x').collect();
    if parts.len() != 2 {
        return Err("Invalid size format. Use WxH (e.g., 8x4)".to_string());
    }

    let width: usize = parts[0].parse().map_err(|_| "Invalid width")?;
    let height: usize = parts[1].parse().map_err(|_| "Invalid height")?;

    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let mut rng = seed;
    let mut grid = Vec::new();

    for _ in 0..height {
        let mut row = Vec::new();
        for _ in 0..width {
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            let val = ((rng >> 16) & 0xFF) as u8;
            row.push(val);
        }
        grid.push(row);
    }

    // Ensure start is 00 and end is FF
    if !grid.is_empty() && !grid[0].is_empty() {
        grid[0][0] = 0x00;
        let h = grid.len();
        let w = grid[0].len();
        grid[h - 1][w - 1] = 0xFF;
    }

    Ok(grid)
}

fn save_map(grid: &[Vec<u8>], filename: &str) -> io::Result<()> {
    let mut content = String::new();
    for row in grid {
        for (i, &val) in row.iter().enumerate() {
            if i > 0 {
                content.push(' ');
            }
            content.push_str(&format!("{:02X}", val));
        }
        content.push('\n');
    }
    fs::write(filename, content)
}

fn get_color_code(val: u8) -> u8 {
    // Rainbow gradient: red -> orange -> yellow -> green -> cyan -> blue -> purple
    match val {
        0x00..=0x24 => 1, // Red
        0x25..=0x48 => 3, // Orange/Yellow
        0x49..=0x6C => 2, // Green
        0x6D..=0x90 => 6, // Cyan
        0x91..=0xB4 => 4, // Blue
        0xB5..=0xD8 => 5, // Magenta
        0xD9..=0xFF => 7, // White
    }
}

fn visualize_map(grid: &[Vec<u8>], path: &[(usize, usize)], max_path: Option<&[(usize, usize)]>) {
    let path_set: HashMap<(usize, usize), bool> = path.iter().map(|&p| (p, true)).collect();
    let max_path_set: HashMap<(usize, usize), bool> = max_path
        .map(|p| p.iter().map(|&pos| (pos, true)).collect())
        .unwrap_or_default();

    // Display full hexadecimal grid
    println!("\nHEXADECIMAL GRID (rainbow gradient):");
    println!("==========================================");
    println!();

    for row in grid.iter() {
        for &val in row.iter() {
            let color = get_color_code(val);
            print!("\x1b[3{}m{:02X}\x1b[0m ", color, val);
        }
        println!();
    }

    // Display minimum path
    println!("\nMINIMUM COST PATH (shown in WHITE):");
    println!("==========================================");
    println!();

    for (y, row) in grid.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            if path_set.contains_key(&(y, x)) {
                print!("\x1b[37m{:02X}\x1b[0m ", val); // White
            } else {
                let color = get_color_code(val);
                print!("\x1b[3{}m{:02X}\x1b[0m ", color, val);
            }
        }
        println!();
    }

    // Display maximum path if present
    if max_path.is_some() {
        println!("\nMAXIMUM COST PATH (shown in RED):");
        println!("==========================================");
        println!();

        for (y, row) in grid.iter().enumerate() {
            for (x, &val) in row.iter().enumerate() {
                if max_path_set.contains_key(&(y, x)) {
                    print!("\x1b[31m{:02X}\x1b[0m ", val); // Red
                } else {
                    let color = get_color_code(val);
                    print!("\x1b[3{}m{:02X}\x1b[0m ", color, val);
                }
            }
            println!();
        }
    }
}

fn animate_pathfinding(
    grid: &[Vec<u8>],
    step: usize,
    current_pos: (usize, usize),
    cost: u32,
    visited: &[(usize, usize)],
    current_path: &[(usize, usize)],
) {
    let visited_set: HashMap<(usize, usize), bool> = visited.iter().map(|&p| (p, true)).collect();
    let path_set: HashMap<(usize, usize), bool> = current_path.iter().map(|&p| (p, true)).collect();

    println!(
        "\nStep {}: Exploring ({},{}) - cost: {}",
        step, current_pos.1, current_pos.0, cost
    );

    for (y, row) in grid.iter().enumerate() {
        for (x, _val) in row.iter().enumerate() {
            if path_set.contains_key(&(y, x)) {
                print!("[\x1b[32m√\x1b[0m]");
            } else if visited_set.contains_key(&(y, x)) {
                print!("[\x1b[33m*\x1b[0m]");
            } else {
                print!("[ ]");
            }
        }
        println!();
    }

    io::stdout().flush().unwrap();
    thread::sleep(Duration::from_millis(100));
}

type PathResult = (Vec<(usize, usize)>, u32, Vec<(usize, usize)>);

fn dijkstra_min(grid: &[Vec<u8>], animate: bool) -> PathResult {
    let height = grid.len();
    let width = grid[0].len();
    let mut dist = vec![vec![u32::MAX; width]; height];
    let mut prev = vec![vec![None; width]; height];
    let mut heap = BinaryHeap::new();
    let mut visited_order = Vec::new();

    dist[0][0] = grid[0][0] as u32;
    heap.push(State {
        cost: grid[0][0] as u32,
        pos: (0, 0),
    });

    while let Some(State { cost, pos }) = heap.pop() {
        let (y, x) = pos;

        if cost > dist[y][x] {
            continue;
        }

        visited_order.push(pos);

        if animate
            && (visited_order.len() == 1
                || visited_order.len() == 2
                || visited_order.len() % 10 == 0)
        {
            let mut path = Vec::new();
            let mut curr = Some(pos);
            while let Some(p) = curr {
                path.push(p);
                curr = prev[p.0][p.1];
            }
            path.reverse();
            animate_pathfinding(grid, visited_order.len(), pos, cost, &visited_order, &path);
        }

        if y == height - 1 && x == width - 1 {
            if animate {
                let mut path = Vec::new();
                let mut curr = Some(pos);
                while let Some(p) = curr {
                    path.push(p);
                    curr = prev[p.0][p.1];
                }
                path.reverse();
                println!("\nStep {}: Path found!", visited_order.len());
                for (y_grid, row) in grid.iter().enumerate() {
                    for (x_grid, _val) in row.iter().enumerate() {
                        if path.contains(&(y_grid, x_grid)) {
                            print!("[\x1b[32m√\x1b[0m]");
                        } else if visited_order.contains(&(y_grid, x_grid)) {
                            print!("[\x1b[33m*\x1b[0m]");
                        } else {
                            print!("[ ]");
                        }
                    }
                    println!();
                }
            }
            break;
        }

        let mut neighbors = Vec::new();
        if y > 0 {
            neighbors.push((y - 1, x));
        }
        if y + 1 < height {
            neighbors.push((y + 1, x));
        }
        if x > 0 {
            neighbors.push((y, x - 1));
        }
        if x + 1 < width {
            neighbors.push((y, x + 1));
        }

        for &(ny, nx) in &neighbors {
            let new_cost = cost + grid[ny][nx] as u32;

            if new_cost < dist[ny][nx] {
                dist[ny][nx] = new_cost;
                prev[ny][nx] = Some((y, x));
                heap.push(State {
                    cost: new_cost,
                    pos: (ny, nx),
                });
            }
        }
    }

    let mut path = Vec::new();
    let mut curr = Some((height - 1, width - 1));
    while let Some(pos) = curr {
        path.push(pos);
        curr = prev[pos.0][pos.1];
    }
    path.reverse();

    (path, dist[height - 1][width - 1], visited_order)
}

fn dijkstra_max(grid: &[Vec<u8>]) -> (Vec<(usize, usize)>, u32) {
    let height = grid.len();
    let width = grid[0].len();
    let mut dist = vec![vec![0u32; width]; height];
    let mut prev = vec![vec![None; width]; height];
    let mut visited = vec![vec![false; width]; height];
    let mut heap = BinaryHeap::new();

    dist[0][0] = grid[0][0] as u32;
    heap.push(StateMax {
        cost: grid[0][0] as u32,
        pos: (0, 0),
    });

    while let Some(StateMax { cost, pos }) = heap.pop() {
        let (y, x) = pos;

        if visited[y][x] {
            continue;
        }
        visited[y][x] = true;

        if y == height - 1 && x == width - 1 {
            break;
        }

        let mut neighbors = Vec::new();
        if y > 0 {
            neighbors.push((y - 1, x));
        }
        if y + 1 < height {
            neighbors.push((y + 1, x));
        }
        if x > 0 {
            neighbors.push((y, x - 1));
        }
        if x + 1 < width {
            neighbors.push((y, x + 1));
        }

        for &(ny, nx) in &neighbors {
            if visited[ny][nx] {
                continue;
            }

            let new_cost = cost + grid[ny][nx] as u32;

            if new_cost > dist[ny][nx] {
                dist[ny][nx] = new_cost;
                prev[ny][nx] = Some((y, x));
                heap.push(StateMax {
                    cost: new_cost,
                    pos: (ny, nx),
                });
            }
        }
    }

    let mut path = Vec::new();
    let mut curr = Some((height - 1, width - 1));
    while let Some(pos) = curr {
        path.push(pos);
        curr = prev[pos.0][pos.1];
    }
    path.reverse();

    (path, dist[height - 1][width - 1])
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let grid = if let Some(ref gen_size) = args.generate {
        let parts: Vec<&str> = gen_size.split('x').collect();
        println!("Generating {}x{} hexadecimal grid...", parts[0], parts[1]);
        println!();
        let grid = generate_map(gen_size.clone())?;

        if let Some(output_file) = &args.output {
            save_map(&grid, output_file).map_err(|e| format!("Failed to save map: {}", e))?;
            println!("Map saved to: {}", output_file);
        }

        println!("Generated Map:");
        for row in &grid {
            for &val in row {
                print!("{:02X} ", val);
            }
            println!();
        }
        println!();

        grid
    } else if let Some(map_file) = args.map_file {
        let content =
            fs::read_to_string(&map_file).map_err(|e| format!("Failed to read map file: {}", e))?;
        parse_map(&content)?
    } else {
        return Err("Either provide a map file or use --generate".to_string());
    };

    if args.animate {
        println!("Searching for minimum cost path...");
    } else if args.generate.is_some() {
        println!("Finding optimal paths...");
    }

    let (min_path, min_cost, _visited) = dijkstra_min(&grid, args.animate);

    if !args.animate {
        println!();
    }

    println!("MINIMUM COST PATH");
    if args.visualize && !args.both {
        visualize_map(&grid, &min_path, None);
        println!("\nCost: {} (minimum)", min_cost);
    } else if !args.visualize {
        println!("\nMinimum cost path: {}", min_cost);
        print!("Path: ");
        for (i, &(y, x)) in min_path.iter().enumerate() {
            if i > 0 {
                print!(" → ");
            }
            print!("({},{})", x, y);
        }
        println!();
    }

    if args.both {
        let (max_path, max_cost) = dijkstra_max(&grid);
        if args.visualize {
            visualize_map(&grid, &min_path, Some(&max_path));
            println!("\nCost: {} (minimum)", min_cost);
            println!("\nCost: {} (maximum)", max_cost);
        } else {
            println!("\nMaximum cost path: {}", max_cost);
            print!("Path: ");
            for (i, &(y, x)) in max_path.iter().enumerate() {
                if i > 0 {
                    print!(" → ");
                }
                print!("({},{})", x, y);
            }
            println!();
        }
    }

    if args.animate {
        println!();
    }

    Ok(())
}
