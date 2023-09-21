#![allow(non_snake_case)]

use std::{
    collections::HashSet,
    env,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    io::stdin,
    thread, time::Duration
};

////////////////////////////////////////////////////////////////////////////////

pub const BLK1 :char = '◆';
pub const BLK2 :char = '●';
pub const BLK3 :char = '▮';
pub const BLK  :char = '◼';

pub const SAV: &str = "\x1b7";
pub const RES: &str = "\x1b8";
pub const CLR: &str = "\x1b[2J";
pub const HOM: &str = "\x1b[H";
pub const RST: &str = "\x1b[0m";
pub const IBLK: &str = "\x1b[1;30m";
pub const IRED: &str = "\x1b[1;31m";
pub const IYEL: &str = "\x1b[1;33m";
pub const FLS: &str = "\x1b[5m";

pub type Res<T> = Result<T, Box<dyn Error>>;

pub fn readline() -> String {
    let mut buff = String::new();
    stdin().read_line(&mut buff).ok();
    buff
}

pub fn sleep (secs: f64)  {
    thread::sleep(Duration::from_millis( (secs*1000.0) as u64));
}

#[macro_export]
macro_rules! IF {
    ($p:expr, $t:expr, $f:expr) => (if $p { $t } else { $f })
}


// Terminal //////////////////////////////////////////////////////////

#[derive(Debug)]
struct Term {
    h:usize,
    w:usize
}

impl Term {
    fn new() -> Term {
        let mut args = env::args().skip(1).take(2).flat_map(|s| s.parse::<usize>());
        Term {
            h: args.next().unwrap_or(25),
            w: args.next().unwrap_or(80)
        }
    }
}

// Point //////////////////////////////////////////////////////////

#[derive(Eq, Clone, Hash, PartialEq)]
pub struct Point {
    y: usize,
    x: usize
}

impl Point {
    fn new (y: usize, x: usize) -> Point { Point{y, x} }
}

impl Debug for Point {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str(&format!("({},{})", self.y, self.x))
    }
}

////////////////////////////////////////

pub struct Glyph {
    color: String,
    glyph: String
}

impl Glyph {
    fn new (color: String, glyph: String) -> Glyph {
        Glyph{color, glyph}
    }
    fn glyph(&self) -> String {
        format!("{}{}", self.color, self.glyph)
    }
}

////////////////////////////////////////

// AKA Eigenstate
pub struct State {
    pub id: usize,
    glyph: Glyph,
    projections: Vec<SuperState> // Superstates allowed for each direction
}

impl State {
    fn new(id: usize, (clr, glf): (&str, &str), projections: &[&[usize]]) -> State {
        State{
            id,
            glyph: Glyph::new(clr.to_string(), glf.to_string()),
            projections: projections.iter()
                .map(|states| SuperState::from(states.iter().map(|i|*i)))
                .collect()
        }
    }
}

////////////////////////////////////////

struct SuperState {
    states: HashSet<usize>
}

impl SuperState {
    fn from(states: impl Iterator<Item=usize>) -> SuperState {
        SuperState{states: states.collect()}
    }
    fn intersect(&self, hss: &HashSet<usize>) -> HashSet<usize> {
        &self.states & hss
    }
    fn count(&self) -> usize {
        self.states.len()
    }
    fn states(&self) -> impl Iterator<Item=usize> + '_{
        self.states.iter().map(|i|*i)
    }
    fn state(&self) -> usize {
        *(self.states.iter().next().expect("superstate is empty"))
    }
    fn collapse(&mut self) {
        let i = *self.states.iter().next().expect("superstate empty");
        self.states.clear();
        self.states.insert(i);
    }
}

impl Debug for SuperState {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str(&format!("{:?}", self.states))
    }
}
////////////////////////////////////////

pub struct WaveFunction {
    info: usize,
    term: Term,
    top: usize,
    cursor: (Point, Point),
    basestates: Vec<State>, // fixed vector of basis states
    lastColor: String,
    grid: Vec<Vec<SuperState>>, // Grid of states (state == one or more possible values)
    rowcount: Vec<usize>,
    groups: Vec<HashSet<Point>> // Group values by wave count
}

impl WaveFunction {
    fn new (basestates: Vec<State>) -> WaveFunction {
        let term = Term::new();
        let numStates = basestates.len();
        let mut groups: Vec<HashSet<_>> = (0..=numStates)
            .into_iter()
            .map(|_| HashSet::new())
            .collect();
        WaveFunction{
            info: 0,
            top: 0,
            cursor: (Point::new(0, 0), Point::new(0, 0)),
            basestates,
            lastColor: String::new(),
            grid: (0..term.h).into_iter()
                .map(|y| (0..term.w).into_iter()
                    .map(|x| {
                        groups[numStates].insert(Point::new(y, x));
                        SuperState::from(0..numStates)
                    }).collect())
                .collect(),
            rowcount: (0..term.h).map(|_|0).collect(),
            groups,
            term
        }
    }
    fn resetrow (&mut self) {
        self.top = (self.top + self.term.h - 1) % self.term.h;
        let row = self.top;
        let numStates = self.basestates.len();
        (0..self.term.w).into_iter().for_each(|x| {
            let p = Point::new(row, x);
            self.groups[1].remove(&p);
            self.groups[numStates].insert(p);
            self.grid[row][x].states = (0..numStates).collect();
        });
        self.rowcount[row] = 0;
        (0..self.term.w).into_iter().for_each(|x| {
            let row = (row + 1) % self.term.h;
            let p = Point::new(row, x);
            self.projectdir(((p.y+self.term.h-1)%self.term.h, x), &p, 0);
        });
    }
    // Get superstate at point
    fn ss (&mut self, p: &Point) -> &mut SuperState { &mut self.grid[p.y][p.x] }
    fn ss_ref (&self, p: &Point) -> &    SuperState { &    self.grid[p.y][p.x] }
    // Projection at location/direction:  Allowed states in that neighbor
    fn projection_ss (&self, h: &HashSet<usize>, dir: usize) -> HashSet<usize> {
        h.iter().map(|i|*i)
            .flat_map(|id| self.basestates[id].projections[dir].states())
            .collect::<HashSet<usize>>()
    }
    fn is_superpositioned (&self, p: &Point) -> bool {
        2 <= self.ss_ref(p).count()
    }
    fn projectdir(&mut self, (y,x): (usize, usize), op: &Point, dir: usize) -> Option<()> {
        let p = Point::new(y, x);
        self.cursor = (p.clone(), op.clone());
        let sscount = self.ss(&p).count();
        if sscount < 2 { return Some(()) } // Skip already collapsed state

        // The "top row is ignored...disables y-axis torus mapping.
        //if (op.y==self.top && 0==dir) || ((op.y+1)%self.term.h==self.top && 1==dir) { return }

        let hashset2 = self.ss_ref(&p).intersect(&self.projection_ss(&self.ss_ref(&op).states, dir));
        let sscountfinal = hashset2.len();

        if sscount != sscountfinal {
            self.ss(&p).states.clear();
            self.ss(&p).states = hashset2;
            if 1 == sscountfinal {
                self.rowcount[p.y] += 1;
            }
            self.plotGlyph(&p);
            self.groups[sscount].remove(&p);
            self.groups[sscountfinal].insert(p.clone());
            return match sscountfinal {
                0 => Some(()),
                _ => self.projectState(&p)
            }
        }
        Some(())
    }
    fn projectState(&mut self, p: &Point) -> Option<()> {
        let y = p.y;
        let x = p.x;
        self.projectdir(((y+self.term.h-1)%self.term.h, x), p, 0)?;
        self.projectdir(((y+1)            %self.term.h, x), p, 1)?;
        self.projectdir((y, (x+1)            %self.term.w), p, 2)?;
        self.projectdir((y, (x+self.term.w-1)%self.term.w), p, 3)
    }
    fn collapseAt(&mut self, p: &Point) -> Option<()> {
        assert!(self.is_superpositioned(p)); // Should only collapse superstates
        self.rowcount[p.y] += 1;
        self.grid[p.y][p.x].collapse();
        self.plotGlyph(p);
        self.projectState(p)
    }
    fn getLowestEntropy(&mut self) -> Option<Point> {
        self.groups.iter_mut()
            .skip(2)
            .find(|h| 0<h.len())
            .map(|h| h.take(&h.iter().next().expect("impossible").clone()).expect("not possible"))
            .map(|p| { self.groups[1].insert(p.clone()); p })
    }
    pub fn collapseMaybe(&mut self) -> bool {
        match self.getLowestEntropy() {
            Some(p) => { match self.collapseAt(&p) { Some(_) => true, None => false} }
            None => false
        }
    }
    pub fn stateAt (&self, p: &Point) -> usize {
        self.grid[p.y][p.x].state()
    }
    pub fn glyphAt (&self, p: &Point) -> &Glyph {
        &self.basestates[self.stateAt(p)].glyph
    }
    pub fn plotGlyph(&mut self, p: &Point) {
        match self.ss_ref(&p).states.len() {
        0 => {
            self.lastColor = format!("{FLS}{IYEL}").to_string();
            print!("\x1b[{};{}H{RST}{}!", p.y+1, p.x+1, self.lastColor);
        }
        1 => {
            let clr = &self.glyphAt(p).color;
            if &self.lastColor != clr {
                self.info += 1;
                self.lastColor = self.glyphAt(p).color.to_string();
                print!("{RST}{}", self.lastColor);
            }
            print!("\x1b[{};{}H{}", p.y+1, p.x+1, self.glyphAt(p).glyph);
            //print!("\x1b[H\n"); readline();
        },
        c => {
            self.lastColor = format!("{IBLK}").to_string();
            print!("\x1b[{};{}H{RST}{}{:x}", p.y+1, p.x+1, self.lastColor, c);
        }
        }
    }
    pub fn print (&self) -> &Self { print!("{}\x1b[0m", self); self }
    pub fn printTop (&self) -> &Self {
        let y = self.top;
        let r = &self.grid[y];
        r.iter().for_each(|ss| {
            match ss.states.len() {
                0 => print!("     "),
                1 => print!("{}", self.basestates[*ss.states.iter().next().expect("should not occur")].glyph.glyph()),
                l => print!("{}", l)
            };
        });
        print!("\n");
        self
    }
    pub fn debug (&self) -> &Self { print!("{:?}\x1b[0m", self); self }
}

impl Debug for WaveFunction {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.grid.iter().enumerate().for_each(|(y, r)| {
            r.iter().enumerate().for_each(|(x, ss)| {
                let p = Point::new(y,x);
                let mut s = 0;
                if self.cursor.1 == p {
                    fmt.write_str(&"\x1b[44m" )
                } else if self.cursor.0 == p {
                    fmt.write_str(&"\x1b[42m" )
                } else {
                    fmt.write_str(&"\x1b[100m" )
                }.ok();
                if 1 == ss.states.len() { fmt.write_str("\x1b[0;1m").ok(); }
                fmt.write_str(if ss.states.get(&0).is_some() { &" " } else { s+=1; &"" } ).ok();
                fmt.write_str(if ss.states.get(&1).is_some() { &"+" } else { s+=1; &"" } ).ok();
                fmt.write_str(if ss.states.get(&2).is_some() { &"-" } else { s+=1; &"" } ).ok();
                fmt.write_str(if ss.states.get(&3).is_some() { &"|" } else { s+=1; &"" } ).ok();
                fmt.write_str(if ss.states.get(&4).is_some() { &"#" } else { s+=1; &"" } ).ok();
                fmt.write_str(&"\x1b[0m " ).ok();
                fmt.write_str(&"     "[0..s] ).ok();
            });
            fmt.write_str("\n").ok();
        });
        self.groups.iter().for_each(|hs| { fmt.write_str(&format!("{} {:?}\n", hs.len(), hs)).ok(); });
        Ok(())
    }
}

impl Display for WaveFunction {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        (0..self.term.h).into_iter().for_each(|y| {
            let y = (y + self.top) % self.term.h;
            let r = &self.grid[y];
            //fmt.write_str(&format!("{:3} ", self.rowcount[y])).ok();
            r.iter().for_each(|ss| {
                match ss.states.len() {
                    1 => fmt.write_str(&self.basestates[*ss.states.iter().next().expect("can not occur")].glyph.glyph()),
                    l => fmt.write_str(&format!("\x1b[0m{}", l))
                }.ok();
            });
            if y < self.term.h-1 { fmt.write_str("\n").ok(); }
        });
        Ok(())
    }
}


// Main //////////////////////////////////////////////////////////////

pub fn header () {
    println!("\x1b[31m__        _______ ____ ");
    println!("\x1b[33m\\ \\      / /  ___/ ___|");
    println!("\x1b[32m \\ \\ /\\ / /| |_ | |    ");
    println!("\x1b[34m  \\ V  V / |  _|| |___ ");
    println!("\x1b[35m   \\_/\\_/  |_|   \\____|\x1b[0m");
}

pub fn maze () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0, ("\x1b[1;30;40m","."), &[&[0,2,4,5,6,9],&[0,2,4,7,8,11],&[0,1,3,5,8,12],&[0,1,3,6,7,10]]),

        State::new(1, ("\x1b[1;31;40m","|"), &[&[1,3,7,8,10,11,12],&[1,3,5,6,9,10,12],&[0],&[0]]),
        State::new(2, ("\x1b[1;31;40m","-"), &[&[0],&[0],&[2,4,6,7,9,10],&[2,4,5,8,9,12]]),

        // |
        State::new(3, ("\x1b[1;31;40m","|"), &[&[1],&[1],&[0],&[0]]),
        // -
        State::new(4, ("\x1b[1;31;40m","-"), &[&[0],&[0],&[2],&[2]]),

        // L
        State::new(5, ("\x1b[1;31;40m","#"), &[&[1],&[0],&[2],&[0]]),
        // _|
        State::new(6, ("\x1b[1;31;40m","#"), &[&[1],&[0],&[0],&[2]]),
        // 7
        State::new(7, ("\x1b[1;31;40m","#"), &[&[0],&[1],&[0],&[2]]),
        // |~
        State::new(8, ("\x1b[1;31;40m","#"), &[&[0],&[1],&[2],&[0]]),

        // _|_
        State::new(9, ("\x1b[1;31;40m","-"), &[&[1],&[0],&[2],&[2]]),
        // -|
        State::new(10, ("\x1b[1;31;40m","|"), &[&[1],&[1],&[0],&[2]]),
        // ^|^
        State::new(11, ("\x1b[1;31;40m","-"), &[&[0],&[1],&[2],&[2]]),
        //  |-
        State::new(12, ("\x1b[1;31;40m","|"), &[&[1],&[1],&[2],&[0]]),

    ));
    while wf.collapseMaybe() { }
    print!("{HOM}{RST}\n");
    if !true {
        print!("\x1b[H");
        wf.print();
        wf.resetrow();
        for _ in 0..100 {
            while wf.collapseMaybe() { }
            print!("\x1b[H\x1bM");
            wf.printTop();
            wf.resetrow();
        }
    }
    wf
}

pub fn maze0 () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0, ("\x1b[0;40m",   " "), &[&[0,2,4,7,8,9,15,16,17],&[0,2,4,5,6,11,14,16,17],&[0,1,3,5,7,12,14,15,16],&[0,1,3,6,8,10,14,15,17]]),
        // |
        State::new(1, ("\x1b[44;1;34m","|"), &[&[3,5,6,10,11,12,13,14],&[3,7,8,9,10,12,13,15],&[0],&[0]]),
        // -
        State::new(2, ("\x1b[44;1;34m","-"), &[&[0],&[0],&[4,6,8,9,10,11,13,17],&[4,5,7,9,11,12,13,16]]),
        // |
        // *
        // |
        State::new(3, ("\x1b[44;1;34m","+"), &[&[1],&[1],&[0],&[0]]),
        // -*-
        State::new(4, ("\x1b[44;1;34m","+"), &[&[0],&[0],&[2],&[2]]),
        // *-
        // |
        State::new(5, ("\x1b[44;1;34m","+"), &[&[0],&[1],&[2],&[0]]),
        // -*
        //  |
        State::new(6, ("\x1b[44;1;34m","+"), &[&[0],&[1],&[0],&[2]]),
        //  |
        //  *-
        State::new(7, ("\x1b[44;1;34m","+"), &[&[1],&[0],&[2],&[0]]),
        //  |
        // -*
        State::new(8, ("\x1b[44;1;34m","+"), &[&[1],&[0],&[0],&[2]]),
        //  |
        // -*-
        State::new(9, ("\x1b[44;1;34m","+"), &[&[1],&[0],&[2],&[2]]),
        //  |
        // -*
        //  |
        State::new(10, ("\x1b[44;1;34m","+"), &[&[1],&[1],&[0],&[2]]),
        // -*-
        //  |
        State::new(11, ("\x1b[44;1;34m","+"), &[&[0],&[1],&[2],&[2]]),
        //  |
        //  *-
        //  |
        State::new(12, ("\x1b[44;1;34m","+"), &[&[1],&[1],&[2],&[0]]),
        //  |
        // -*-
        //  |
        State::new(13, ("\x1b[44;1;34m","+"), &[&[1],&[1],&[2],&[2]]),
        //  *
        //  |
        State::new(14, ("\x1b[44;1;34m","+"), &[&[0],&[1],&[0],&[0]]),
        //  |
        //  *
        State::new(15, ("\x1b[44;1;34m","+"), &[&[1],&[0],&[0],&[0]]),
        //  *-
        State::new(16, ("\x1b[44;1;34m","+"), &[&[0],&[0],&[2],&[0]]),
        //  -*
        State::new(17, ("\x1b[44;1;34m","+"), &[&[0],&[0],&[2],&[0]]),

    ));
    while wf.collapseMaybe() { }
    print!("{HOM}{RST}\n");
    if !true {
        print!("\x1b[H");
        wf.print();
        wf.resetrow();
        for _ in 0..100 {
            while wf.collapseMaybe() { }
            print!("\x1b[H\x1bM");
            wf.printTop();
            wf.resetrow();
        }
    }
    wf
}



pub fn ultima () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0, ("\x1b[0;34m", &BLK.to_string()), &[&[0,1],  &[0,1],  &[0,1],  &[0,1]]),
        State::new(1, ("\x1b[1;34m", &BLK.to_string()), &[&[0,1,2],&[0,1,2],&[0,1,2],&[0,1,2]]),
        State::new(2, ("\x1b[0;33m", &BLK.to_string()), &[&[1,2,3],&[1,2,3],&[1,2,3],&[1,2,3]]),
        State::new(3, ("\x1b[1;32m", &BLK.to_string()), &[&[2,3,4],&[2,3,4],&[2,3,4],&[2,3,4]]),
        State::new(4, ("\x1b[0;37m", &BLK.to_string()), &[&[3,4,5],&[3,4,5],&[3,4,5],&[3,4,5]]),
        State::new(5, ("\x1b[1;37m", &BLK.to_string()), &[&[4,5],  &[4,5],  &[4,5],  &[4,5]]),
    ));
    while wf.collapseMaybe() { }
    print!("{HOM}{RST}\n");
    if !true {
        print!("\x1b[H");
        wf.print();
        wf.resetrow();
        for _ in 0..100 {
            while wf.collapseMaybe() { }
            if !true {
                print!("\x1b[H\x1bM");
                wf.printTop();
                wf.resetrow();
           }
        }
    }
    wf
}

pub fn mobo () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        // outside space D A/B C
        State::new(0, ("\x1b[42;32m"," "), &[&[0,3,4,6,11],&[0,1,2,5,11],&[0,1,3,8,10],&[0,2,4,7,10]]),

        // left upper corner
        State::new(1, ("\x1b[1;42;37m","="), &[&[0],&[8],&[5],&[0]]),
        //             right upper corner
        State::new(2, ("\x1b[1;42;37m","="), &[&[0],&[7],&[0],&[5]]),
        // left lower corner
        State::new(3, ("\x1b[1;42;37m","="), &[&[8],&[0],&[6],&[0]]),
        //             right lower corner
        State::new(4, ("\x1b[1;42;37m","="), &[&[7],&[0],&[0],&[6]]),

        //     Upper wall
        State::new(5, ("\x1b[1;40;31m"," "), &[&[0,10],&[9],&[2,5],&[1,5]]),
        //     Lower wall
        State::new(6, ("\x1b[1;40;31m"," "), &[&[9],&[0,10],&[4,6],&[3,6]]),
        //         Right wall
        State::new(7, ("\x1b[1;42;37m","="), &[&[2,7],&[4,7],&[0,11],&[9]]),
        // Left wall
        State::new(8, ("\x1b[1;42;37m","="), &[&[1,8],&[3,8],&[9],&[0,11]]),

        // inside space            D A/B C
        State::new(9, ("\x1b[0;40;31m"," "), &[&[5,9],&[6,9],&[7,9],&[8,9]]),

        // verticle path
        State::new(10, ("\x1b[1;42;32m","|"), &[&[6,10,12],&[5,10,12],&[0],&[0]]),

        // horizontal path
        State::new(11, ("\x1b[1;42;32m","-"), &[&[0],&[0],&[8,11,12],&[7,11,12]]),

        // crossroad path
        State::new(12, ("\x1b[1;42;32m","+"), &[&[10],&[10],&[11],&[11]]),
    ));
    while wf.collapseMaybe() {  }
    print!("{HOM}{RST}\n");
    if !true {
        wf.resetrow();
        for _ in 0..100 {
            while wf.collapseMaybe() { }
            if true {
                print!("\x1b[H\x1bM");
                wf.printTop();
                wf.resetrow();
                readline();
            }
        }
    }
    wf
}
pub fn rogue () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        // outside space D A/B C
        State::new(0, ("\x1b[0;40;32m",":"), &[&[0,3,4,6,11],&[0,1,2,5,11],&[0,1,3,8,10],&[0,2,4,7,10]]),

        // left upper corner
        State::new(1, ("\x1b[40;1;31m","#"), &[&[0],&[8],&[5],&[0]]),
        //             right upper corner
        State::new(2, ("\x1b[40;1;31m","#"), &[&[0],&[7],&[0],&[5]]),
        // left lower corner
        State::new(3, ("\x1b[40;1;31m","#"), &[&[8],&[0],&[6],&[0]]),
        //             right lower corner
        State::new(4, ("\x1b[40;1;31m","#"), &[&[7],&[0],&[0],&[6]]),

        //     Upper wall
        State::new(5, ("\x1b[1;40;31m","-"), &[&[0,10],&[9],&[2,5],&[1,5]]),
        //     Lower wall
        State::new(6, ("\x1b[1;40;31m","-"), &[&[9],&[0,10],&[4,6],&[3,6]]),
        //         Right wall
        State::new(7, ("\x1b[1;40;31m","|"), &[&[2,7],&[4,7],&[0,11],&[9]]),
        // Left wall
        State::new(8, ("\x1b[1;40;31m","|"), &[&[1,8],&[3,8],&[9],&[0,11]]),

        // inside space            D A/B C
        State::new(9, ("\x1b[1;40;30m","@"), &[&[5,9],&[6,9],&[7,9],&[8,9]]),

        // verticle path
        State::new(10, ("\x1b[0;40;36m","#"), &[&[6,10,12],&[5,10,12],&[0],&[0]]),

        // horizontal path
        State::new(11, ("\x1b[1;40;34m","="), &[&[0],&[0],&[8,11,12],&[7,11,12]]),

        // crossroad path
        State::new(12, ("\x1b[0;40;36m","#"), &[&[10],&[10],&[11],&[11]]),
    ));
    while wf.collapseMaybe() {  }
    print!("{HOM}{RST}\n");
    if !true {
        for _ in 0..100 {
            wf.resetrow();
            while wf.collapseMaybe() { }
            if true {
                print!("\x1b[H\x1bM");
                wf.printTop();
                //readline();
                //sleep(0.2);
            }
        }
    }
    wf
}

pub fn main () {
    print!("USAGE:  wavefunctioncollapse [HEIGHT default 25] [WIDTH default 80]");
    print!("{SAV}{HOM}{CLR}");
    header();
    //print!("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n", (wf.term.w+1)*wf.term.h);
    loop {
        maze(); sleep(3.0);
        maze0(); sleep(3.0);
        ultima(); sleep(3.0);
        mobo(); sleep(3.0);
        rogue(); sleep(3.0);
    }
    //print!("\x1b[H{}\r", wf);
    //print!("\x1b[{}H\x1b[1;37;41m{}\x1b[0m", wf.term.h, wf.info);
    //readline();
    //print!("{RES}done.{RST}");
}

/*
 * https://en.wikipedia.org/wiki/Quantum_superposition
 * https://en.wikipedia.org/wiki/Wave_function_collapse

  D A/B C : directions

  Wavefunction Collapse
    Wavefunction - Initial superposition of basis states.  Observing
                   collapses superposition of eigenstates to one.
                   Observing projects the wave function onto a random
                   eigenstate.
    Quantum State - Can be added (like waves) to create new QS

    Collapsing the wavefunction of a single cell.

    Lower entropy -> fewest eigenstates is best wavefunction to collapse

    Eigenstate/Eigenvector - Quantum state with definite value

    Superposition of eigenstates will have various probabilistic eigenvalues

Collapse center, Superpose orthogonals

collapsing a QS will
   superpose itself with each of its orthogonal neighbors
   collapse to a random value
   superpose its neighbors with their orthogonal neighbors

superpose and self with orthogonal states' constraints

*/