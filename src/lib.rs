mod utils;

use crate::utils::set_panic_hook;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

static FOUR9THS: f64 = 4.0 / 9.0;
static ONE9TH: f64 = 1.0 / 9.0;
static ONE36TH: f64 = 1.0 / 36.0;

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    c: f64,
    e: f64,
    n: f64,
    w: f64,
    s: f64,
    ne: f64,
    nw: f64,
    sw: f64,
    se: f64,
}

#[wasm_bindgen]
impl Cell {
    pub fn new(c: f64, e: f64, n: f64, w: f64, s: f64, ne: f64, nw: f64, sw: f64, se: f64) -> Self {
        Self {
            c,
            e,
            n,
            w,
            s,
            ne,
            nw,
            sw,
            se,
        }
    }

    #[inline]
    pub fn rho(&self) -> f64 {
        self.c + self.e + self.n + self.w + self.s + self.ne + self.nw + self.sw + self.se
    }

    #[inline]
    pub fn ux(&self) -> f64 {
        let rho = self.rho();
        if rho != 0.0 {
            (self.e + self.ne + self.se - self.w - self.nw - self.sw) / rho
        } else {
            0.0
        }
    }

    #[inline]
    pub fn uy(&self) -> f64 {
        let rho = self.rho();
        if rho != 0.0 {
            (self.s + self.sw + self.se - self.n - self.ne - self.nw) / rho
        } else {
            0.0
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            c: 0.0,
            e: 0.0,
            n: 0.0,
            w: 0.0,
            s: 0.0,
            ne: 0.0,
            nw: 0.0,
            sw: 0.0,
            se: 0.0,
        }
    }
}

#[wasm_bindgen]
#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum DrawMode {
    Speed = 0,
    XVelocity = 1,
    YVelocity = 2,
    Density = 3,
    Curl = 4,
    Nothing = 5,
}

#[wasm_bindgen]
pub struct Config {
    width: u32,
    height: u32,
    steps_per_frame: u8,
    flow_speed: f64,
    draw_mode: DrawMode,
    omega: f64,
}

#[wasm_bindgen]
impl Config {
    pub fn new(
        width: u32,
        height: u32,
        steps_per_frame: u8,
        flow_speed: f64,
        draw_mode: DrawMode,
        viscosity: f64,
    ) -> Self {
        let mut config = Self {
            width,
            height,
            steps_per_frame,
            flow_speed,
            draw_mode,
            omega: 0.0,
        };
        config.update_viscosity(viscosity);

        config
    }

    pub fn update_viscosity(&mut self, viscosity: f64) {
        let omega = 1.0 / (3.0 * viscosity + 0.5);
        self.omega = omega;
    }
}

#[wasm_bindgen]
pub struct Lattice {
    config: Config,
    // As an optimization, we switch back and forth between these two vectors,
    // streaming data from one to the other, and vice versa.
    cells_one: Vec<Cell>,
    cells_two: Vec<Cell>,
    use_one: bool,
    density: Vec<f64>,
    ux: Vec<f64>,
    uy: Vec<f64>,
    barrier_set: HashSet<usize>,
    barrier: Vec<usize>,
    curl: Vec<f64>,
    // TODO: It would be nice to package these together...
    // doing so efficiently, though, may be tricky
    flow_particles_x: Vec<f64>,
    flow_particles_y: Vec<f64>,
}

#[wasm_bindgen]
impl Lattice {
    pub fn new(config: Config) -> Self {
        set_panic_hook();
        let cells_one = make_vec(config.width, config.height);
        let cells_two = make_vec(config.width, config.height);
        let density = make_vec(config.width, config.height);
        let ux = make_vec(config.width, config.height);
        let uy = make_vec(config.width, config.height);
        let curl = make_vec(config.width, config.height);
        let barrier_set = HashSet::new();
        let barrier = vec![];
        let mut lattice = Self {
            cells_one,
            cells_two,
            use_one: true,
            config,
            density,
            ux,
            uy,
            barrier_set,
            barrier,
            curl,
            flow_particles_x: vec![],
            flow_particles_y: vec![],
        };
        lattice.init_flow(0.0, 0.0, 1.0);

        lattice
    }

    fn init_flow(&mut self, ux: f64, uy: f64, rho: f64) {
        let ux3 = 3.0 * ux;
        let uy3 = 3.0 * -uy;
        let ux2 = ux * ux;
        let uy2 = -uy * -uy;
        let uxuy2 = 2.0 * ux * -uy;
        let u2 = ux2 + uy2;
        let u215 = 1.5 * u2;

        let c = FOUR9THS * rho * (1.0 - u215);
        let e = ONE9TH * rho * (1.0 + ux3 + 4.5 * ux2 - u215);
        let n = ONE9TH * rho * (1.0 + uy3 + 4.5 * uy2 - u215);
        let w = ONE9TH * rho * (1.0 - ux3 + 4.5 * ux2 - u215);
        let s = ONE9TH * rho * (1.0 - uy3 + 4.5 * uy2 - u215);
        let ne = ONE36TH * rho * (1.0 + ux3 + uy3 + 4.5 * (u2 + uxuy2) - u215);
        let nw = ONE36TH * rho * (1.0 - ux3 + uy3 + 4.5 * (u2 - uxuy2) - u215);
        let sw = ONE36TH * rho * (1.0 - ux3 - uy3 + 4.5 * (u2 + uxuy2) - u215);
        let se = ONE36TH * rho * (1.0 + ux3 - uy3 + 4.5 * (u2 - uxuy2) - u215);

        let wid = self.config.width;
        let hei = self.config.height;
        let barrier_set = &self.barrier_set;

        let cells = if self.use_one {
            &mut self.cells_one
        } else {
            &mut self.cells_two
        };

        for i in 0..(wid * hei) as usize {
            if !barrier_set.contains(&i) {
                self.density[i] = rho;
                self.ux[i] = ux;
                self.uy[i] = uy;
                cells[i].c = c;
                cells[i].e = e;
                cells[i].n = n;
                cells[i].w = w;
                cells[i].s = s;
                cells[i].ne = ne;
                cells[i].nw = nw;
                cells[i].sw = sw;
                cells[i].se = se;
            }
        }
    }
    fn set_boundaries(&mut self) {
        // Copied from Daniel V. Schroeder.
        let ux = self.config.flow_speed;
        let uy = 0.0;
        let rho = 1.0;
        let ux3 = 3.0 * ux;
        let uy3 = 3.0 * -uy;
        let ux2 = ux * ux;
        let uy2 = -uy * -uy;
        let uxuy2 = 2.0 * ux * -uy;
        let u2 = ux2 + uy2;
        let u215 = 1.5 * u2;
        let zero = FOUR9THS * rho * (1.0 - u215);
        let one = ONE9TH * rho * (1.0 + ux3 + 4.5 * ux2 - u215);
        let two = ONE9TH * rho * (1.0 + uy3 + 4.5 * uy2 - u215);
        let three = ONE9TH * rho * (1.0 - ux3 + 4.5 * ux2 - u215);
        let four = ONE9TH * rho * (1.0 - uy3 + 4.5 * uy2 - u215);
        let five = ONE36TH * rho * (1.0 + ux3 + uy3 + 4.5 * (u2 + uxuy2) - u215);
        let six = ONE36TH * rho * (1.0 - ux3 + uy3 + 4.5 * (u2 - uxuy2) - u215);
        let seven = ONE36TH * rho * (1.0 - ux3 - uy3 + 4.5 * (u2 + uxuy2) - u215);
        let eight = ONE36TH * rho * (1.0 + ux3 - uy3 + 4.5 * (u2 - uxuy2) - u215);

        let cells = if self.use_one {
            &mut self.cells_one
        } else {
            &mut self.cells_two
        };

        for x in 0..(self.config.width - 1) {
            let idx = x as usize;
            cells[idx].c = zero;
            cells[idx].e = one;
            cells[idx].n = two;
            cells[idx].w = three;
            cells[idx].s = four;
            cells[idx].ne = five;
            cells[idx].nw = six;
            cells[idx].sw = seven;
            cells[idx].se = eight;
            let idx = ((self.config.height - 1) * self.config.width + x) as usize;
            cells[idx].c = zero;
            cells[idx].e = one;
            cells[idx].n = two;
            cells[idx].w = three;
            cells[idx].s = four;
            cells[idx].ne = five;
            cells[idx].nw = six;
            cells[idx].sw = seven;
            cells[idx].se = eight;
        }
        for y in 0..(self.config.height - 1) {
            let idx = (y * self.config.width) as usize;
            cells[idx].c = zero;
            cells[idx].e = one;
            cells[idx].n = two;
            cells[idx].w = three;
            cells[idx].s = four;
            cells[idx].ne = five;
            cells[idx].nw = six;
            cells[idx].sw = seven;
            cells[idx].se = eight;
            let idx = (y * self.config.width + (self.config.width - 1)) as usize;
            cells[idx].c = zero;
            cells[idx].e = one;
            cells[idx].n = two;
            cells[idx].w = three;
            cells[idx].s = four;
            cells[idx].ne = five;
            cells[idx].nw = six;
            cells[idx].sw = seven;
            cells[idx].se = eight;
        }
    }

    fn stream(&mut self) {
        let wid = self.config.width;
        let hei = self.config.height;
        let (cells, buffer) = if self.use_one {
            (&mut self.cells_one, &mut self.cells_two)
        } else {
            (&mut self.cells_two, &mut self.cells_one)
        };

        // Stream data from the primary cells vec into the buffer
        for idx in 0..(wid * hei) as usize {
            let x = idx as u32 % wid;
            let y = idx as u32 / wid;
            if x == 0 || y == 0 || x >= wid - 1 || y >= hei - 1 {
                let cell = cells[idx];
                buffer[idx].c = cell.c;
                buffer[idx].n = cell.n;
                buffer[idx].nw = cell.nw;
                buffer[idx].e = cell.e;
                buffer[idx].ne = cell.ne;
                buffer[idx].s = cell.s;
                buffer[idx].se = cell.se;
                buffer[idx].w = cell.w;
                buffer[idx].sw = cell.sw;
            } else {
                buffer[idx].c = cells[idx].c;
                buffer[idx].n = cells[((y - 1) * wid + x) as usize].n;
                buffer[idx].nw = cells[((y - 1) * wid + (x + 1)) as usize].nw;
                buffer[idx].e = cells[(y * wid + (x - 1)) as usize].e;
                buffer[idx].ne = cells[((y - 1) * wid + (x - 1)) as usize].ne;
                buffer[idx].s = cells[((y + 1) * wid + x) as usize].s;
                buffer[idx].se = cells[((y + 1) * wid + (x - 1)) as usize].se;
                buffer[idx].w = cells[(y * wid + (x + 1)) as usize].w;
                buffer[idx].sw = cells[((y + 1) * wid + (x + 1)) as usize].sw;
            }
        }
        for &idx in self.barrier_set.iter() {
            let x = idx as u32 % wid;
            let y = idx as u32 / wid;
            let cell = cells[idx];
            buffer[(y * wid + (x + 1)) as usize].e = cell.w;
            buffer[((y + 1) * wid + x) as usize].n = cell.s;
            buffer[(y * wid + (x - 1)) as usize].w = cell.e;
            buffer[((y - 1) * wid + x) as usize].s = cell.n;
            buffer[((y + 1) * wid + (x + 1)) as usize].ne = cell.sw;
            buffer[((y + 1) * wid + (x - 1)) as usize].nw = cell.se;
            buffer[((y - 1) * wid + (x - 1)) as usize].sw = cell.ne;
            buffer[((y - 1) * wid + (x + 1)) as usize].se = cell.nw;
        }

        // Switch the primary and the buffer
        self.use_one = !self.use_one;
    }

    fn collide(&mut self) {
        let wid = self.config.width;
        let hei = self.config.height;
        let cells = if self.use_one {
            &mut self.cells_one
        } else {
            &mut self.cells_two
        };
        for idx in 0..(wid * hei) as usize {
            let x = idx as u32 % wid;
            let y = idx as u32 / wid;
            if x == 0 || y == 0 || x >= wid - 1 || y >= hei - 1 {
                continue;
            }
            if !self.barrier_set.contains(&idx) {
                // Calculate macroscopic density (rho) and velocity (ux, uy)
                // Thanks to Daniel V. Schroeder for this optimization
                // http://physics.weber.edu/schroeder/fluids/
                let rho = cells[idx].rho();
                let ux = cells[idx].ux();
                let uy = cells[idx].uy();
                // Update values stored in node.
                self.density[idx] = rho;
                self.ux[idx] = ux;
                self.uy[idx] = uy;
                // Compute curl if it's going to be drawn. Non-edge nodes only.
                if self.config.draw_mode == DrawMode::Curl {
                    self.curl[idx] = self.uy[(y * wid + (x + 1)) as usize]
                        - self.uy[(y * wid + (x - 1)) as usize]
                        - self.ux[((y + 1) * wid + x) as usize]
                        + self.ux[((y - 1) * wid + x) as usize];
                }
                // Set node equilibrium for each velocity
                // Inlining the equilibrium function here provides significant performance improvements
                let ux3 = 3.0 * ux;
                let uy3 = 3.0 * uy;
                let ux2 = ux * ux;
                let uy2 = uy * uy;
                let uxuy2 = 2.0 * ux * uy;
                let u2 = ux2 + uy2;
                let u215 = 1.5 * u2;
                let one9thrho = ONE9TH * rho;
                let one36thrho = ONE36TH * rho;
                let ux3p1 = 1.0 + ux3;
                let ux3m1 = 1.0 - ux3;
                let omega = self.config.omega;
                cells[idx].c =
                    cells[idx].c + (omega * ((FOUR9THS * rho * (1.0 - u215)) - cells[idx].c));
                cells[idx].e = cells[idx].e
                    + (omega * ((one9thrho * (ux3p1 + 4.5 * ux2 - u215)) - cells[idx].e));
                cells[idx].n = cells[idx].n
                    + (omega * ((one9thrho * (1.0 - uy3 + 4.5 * uy2 - u215)) - cells[idx].n));
                cells[idx].w = cells[idx].w
                    + (omega * ((one9thrho * (ux3m1 + 4.5 * ux2 - u215)) - cells[idx].w));
                cells[idx].s = cells[idx].s
                    + (omega * ((one9thrho * (1.0 + uy3 + 4.5 * uy2 - u215)) - cells[idx].s));
                cells[idx].ne = cells[idx].ne
                    + (omega
                        * ((one36thrho * (ux3p1 - uy3 + 4.5 * (u2 - uxuy2) - u215))
                            - cells[idx].ne));
                cells[idx].nw = cells[idx].nw
                    + (omega
                        * ((one36thrho * (ux3m1 - uy3 + 4.5 * (u2 + uxuy2) - u215))
                            - cells[idx].nw));
                cells[idx].sw = cells[idx].sw
                    + (omega
                        * ((one36thrho * (ux3m1 + uy3 + 4.5 * (u2 - uxuy2) - u215))
                            - cells[idx].sw));
                cells[idx].se = cells[idx].se
                    + (omega
                        * ((one36thrho * (ux3p1 + uy3 + 4.5 * (u2 + uxuy2) - u215))
                            - cells[idx].se));
            }
        }
    }

    pub fn init_flow_particles(&mut self) {
        for y in 1..(self.config.height / 10) {
            for x in 1..(self.config.width / 10) {
                if !self
                    .barrier_set
                    .contains(&(((y * 10) * self.config.width + (x * 10)) as usize))
                {
                    self.flow_particles_x.push(x as f64 * 10.0);
                    self.flow_particles_y.push(y as f64 * 10.0);
                }
            }
        }
    }

    fn move_particles(&mut self) {
        for idx in 0..self.flow_particles_x.len() {
            let p_x = self.flow_particles_x[idx];
            let p_y = self.flow_particles_y[idx];
            let lx = p_x.floor() as u32;
            let ly = p_y.floor() as u32;
            if lx < self.config.width && ly < self.config.height {
                let index = (ly * self.config.width + lx) as usize;
                let ux = self.ux[index];
                let uy = self.uy[index];
                self.flow_particles_x[idx] += ux;
                self.flow_particles_y[idx] += uy;
            }
            if self.config.flow_speed > 0.0 && p_x > (self.config.width - 2) as f64 {
                // Wrap particles around to other side of screen
                self.flow_particles_x[idx] = 1.0;
            }
        }
    }

    pub fn update(&mut self) {
        self.set_boundaries();
        for _ in 0..self.config.steps_per_frame {
            self.stream();
            self.collide();
            if self.flow_particles_x.len() > 0 {
                self.move_particles();
            }
        }
    }

    pub fn cells(&mut self) -> *const Cell {
        let cells = if self.use_one {
            &self.cells_one
        } else {
            &self.cells_two
        };
        cells.as_ptr()
    }

    pub fn barrier(&self) -> *const usize {
        self.barrier.as_ptr()
    }

    pub fn ux(&self) -> *const f64 {
        self.ux.as_ptr()
    }

    pub fn uy(&self) -> *const f64 {
        self.uy.as_ptr()
    }

    pub fn curl(&self) -> *const f64 {
        self.curl.as_ptr()
    }

    pub fn flow_particles_x(&self) -> *const f64 {
        self.flow_particles_x.as_ptr()
    }

    pub fn flow_particles_y(&self) -> *const f64 {
        self.flow_particles_y.as_ptr()
    }

    pub fn flow_size(&self) -> usize {
        self.flow_particles_x.len()
    }

    pub fn density(&self) -> *const f64 {
        self.density.as_ptr()
    }

    pub fn draw_mode(&self) -> DrawMode {
        self.config.draw_mode
    }

    pub fn set_draw_mode(&mut self, draw_mode: DrawMode) {
        self.config.draw_mode = draw_mode;
    }

    pub fn set_viscosity(&mut self, viscosity: f64) {
        self.config.update_viscosity(viscosity);
    }

    pub fn set_flow_speed(&mut self, flow_speed: f64) {
        self.config.flow_speed = flow_speed;
    }

    pub fn set_cell(&mut self, new_cell: Cell, idx: usize) {
        let cells = if self.use_one {
            &mut self.cells_one
        } else {
            &mut self.cells_two
        };
        cells[idx] = new_cell;
    }

    pub fn set_steps_per_frame(&mut self, steps_per_frame: u8) {
        self.config.steps_per_frame = steps_per_frame;
    }

    pub fn clear_flow_particles(&mut self) {
        self.flow_particles_x.clear();
        self.flow_particles_y.clear();
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }

    pub fn height(&self) -> u32 {
        self.config.height
    }
}

fn make_vec<T: Default>(width: u32, height: u32) -> Vec<T> {
    let size = width * height;
    let mut vec = Vec::with_capacity(size as usize);
    for _ in 0..size {
        vec.push(Default::default())
    }
    vec
}
