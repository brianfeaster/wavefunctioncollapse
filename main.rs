#![allow(non_snake_case)]

use std::{
    collections::HashSet,
    env,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    io::stdin
};


pub type Res<T> = Result<T, Box<dyn Error>>;

pub fn readline() -> String {
    let mut buff = String::new();
    stdin().read_line(&mut buff).ok();
    buff
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

// AKA Eigenstate
pub struct State {
    pub id: usize,
    glyph: String,
    projections: Vec<SuperState> // Superstates allowed for each direction
}

impl State {
    fn new(id: usize, glyph: &str, projections: &[&[usize]]) -> State {
        State{
            id,
            glyph: glyph.to_string(),
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
    fn intersect(&self, hss: HashSet<usize>) -> HashSet<usize> {
        self.states.intersection(&hss).map(|&i|i).collect()
    }
    fn count(&self) -> usize {
        self.states.len()
    }
    fn states(&self) -> impl Iterator<Item=usize> + '_{
        self.states.iter().map(|i|*i)
    }
    fn state(&self) -> usize {
        *(self.states.iter().next().unwrap())
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
    term: Term,
    top: usize,
    cursor: (Point, Point),
    states: Vec<State>, // fixed vector of basis states
    grid: Vec<Vec<SuperState>>, // Grid of states (state == one or more possible values)
    rowcount: Vec<usize>,
    groups: Vec<HashSet<Point>> // Group values by wave count
}

impl WaveFunction {
    fn new (states: Vec<State>) -> WaveFunction {
        let term = Term::new();
        let numStates = states.len();
        let mut groups: Vec<HashSet<_>> = (0..=numStates)
            .into_iter()
            .map(|_| HashSet::new())
            .collect();
        WaveFunction{
            top: 0,
            cursor: (Point::new(0, 0), Point::new(0, 0)),
            states,
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
        let numStates = self.states.len();
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
    fn projection_ss (&self, p: &Point, dir: usize) -> HashSet<usize> {
        self.grid[p.y][p.x]
            .states()
            .flat_map(|id| self.states[id].projections[dir].states())
            .collect::<HashSet<usize>>()
    }
    fn is_superpositioned (&self, p: &Point) -> bool {
        2 <= self.ss_ref(p).count()
    }
    fn projectdir(&mut self, (y,x): (usize, usize), op: &Point, dir: usize) {
        let p = Point::new(y, x);
        self.cursor = (p.clone(), op.clone());
        let sscount = self.ss(&p).count();
        if sscount < 2 { return } // Skip already collapsed state

        // The "top row is ignored...disables y-axis torus mapping.
        //if (op.y==self.top && 0==dir) || ((op.y+1)%self.term.h==self.top && 1==dir) { return }

        let hashset2 = self.ss_ref(&p).intersect(self.projection_ss(op, dir));
        let sscountfinal = hashset2.len();

        if sscount != sscountfinal {
            self.ss(&p).states.clear();
            self.ss(&p).states = hashset2;
            if 1 == sscountfinal {
                self.rowcount[p.y] += 1;
                self.plotGlyph(&p);
            }
            self.groups[sscount].remove(&p);
            self.groups[sscountfinal].insert(p.clone());
            self.projectState(&p)
        }
    }
    fn projectState(&mut self, p: &Point) {
        let y = p.y;
        let x = p.x;
        self.projectdir(((y+self.term.h-1)%self.term.h, x), p, 0);
        self.projectdir(((y+1)            %self.term.h, x), p, 1);
        self.projectdir((y, (x+1)            %self.term.w), p, 2);
        self.projectdir((y, (x+self.term.w-1)%self.term.w), p, 3);
    }
    fn collapseAt(&mut self, p: &Point) {
        assert!(self.is_superpositioned(p)); // Should only collapse superstates
        self.rowcount[p.y] += 1;
        self.grid[p.y][p.x].collapse();
        self.plotGlyph(p);
        self.projectState(p);
    }
    fn getLowestEntropy(&mut self) -> Option<Point> {
    self.groups.iter_mut()
      .skip(2)
      .find(|v| 0<v.len())
      .map(|h| h.take(&h.iter().next().unwrap().clone()))
      .flatten()
      .map(|p| {
         self.groups[1].insert(p.clone());
         p
      })
  }
    pub fn collapseMaybe(&mut self) -> bool {
        match self.getLowestEntropy() {
            Some(p) => { self.collapseAt(&p); true}
            None => false
        }
    }
    pub fn stateAt (&self, p: &Point) -> usize {
        self.grid[p.y][p.x].state()
    }
    pub fn glyphAt (&self, p: &Point) -> &'_ str {
        &self.states[self.stateAt(p)].glyph
    }
    pub fn plotGlyph(&self, p: &Point) {
        print!("\x1b[{};{}H{}",
            p.y+1,
            p.x+1,
            self.glyphAt(p))
    }
    pub fn print (&self) -> &Self { print!("{}\x1b[0m", self); self }
    pub fn printTop (&self) -> &Self {
        let y = self.top;
        let r = &self.grid[y];
        r.iter().for_each(|ss| {
            match ss.states.len() {
                0 => print!("     "),
                1 => print!("{}", self.states[*ss.states.iter().next().unwrap()].glyph),
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
                    0 => fmt.write_str("     "),
                    1 => fmt.write_str(&self.states[*ss.states.iter().next().unwrap()].glyph),
                    l => fmt.write_str(&format!("{}", l))
                }.ok();
            });
            fmt.write_str("\n").ok();
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

pub fn moneyDungeon () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0,           " ", &[&[0,1,2  ,4],&[0,1,2  ,4],&[0,1,  3,4],&[0,1,  3,4]]),
        State::new(1, "\x1b[1;31m+", &[&[0,    3],&[0,    3],&[0,  2  ],&[0,  2  ]]), // no ++ connections
        State::new(2, "\x1b[1;31m-", &[&[0,     ],&[0,     ],&[  1,2  ],&[  1,2  ]]),
        State::new(3, "\x1b[1;31m|", &[&[  1,   ],&[  1,   ],&[0,    3],&[0,    3]]),
        State::new(4, "\x1b[1;32m$", &[&[0,     ],&[0,     ],&[0,     ],&[0,     ]]),
    ));
    while wf.collapseMaybe() { }
    return wf;
    print!("\x1b[H");
    wf.print();
    wf.resetrow();
    for _ in 0..100 {
        while wf.collapseMaybe() { }
        print!("\x1b[H\x1bM");
        wf.printTop();
        wf.resetrow();
    }
    wf
}

pub fn ultima () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0, "\x1b[44m ", &[&[0,1],  &[0,1],  &[0,1],  &[0,1]]),
        State::new(1, "\x1b[104m ",  &[&[0,1,2],&[0,1,2],&[0,1,2],&[0,1,2]]),
        State::new(2, "\x1b[43m ", &[&[1,2,3],&[1,2,3],&[1,2,3],&[1,2,3]]),
        State::new(3, "\x1b[102m ", &[&[2,3,4],&[2,3,4],&[2,3,4],&[2,3,4]]),
        State::new(4, "\x1b[47m ", &[&[3,4,5],&[3,4,5],&[3,4,5],&[3,4,5]]),
        State::new(5, "\x1b[107m ", &[&[4,5],  &[4,5],  &[4,5],  &[4,5]]),
    ));
    while wf.collapseMaybe() { }
    return wf;
    print!("\x1b[H");
    wf.print();
    wf.resetrow();
    for _ in 0..100 {
        while wf.collapseMaybe() { }
        break;
        print!("\x1b[H\x1bM");
        wf.printTop();
        wf.resetrow();
    }
    wf
}

fn main () {
    header();
    //print!("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n", (wf.term.w+1)*wf.term.h);
    //moneyDungeon();
    ultima();
    println!("\x1b[H\x1b[0m@");
    readline();
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

, collections::HashMap};
   X / \    diagonals 
   < > ^ V  transitions
   + - |    cartesianals
  let hm = HashMap::<usize, usize>::new();
  println!("{:?} {:?}", hm, vec![1,2,3]);

  map z :w!<cr>:!rustc % && ./helloRust $(stty size)<cr>

Collapse center, Superpose orthogonals
         +-|
  +-|    +      +-|
         +-|

   +     +-+ |
   |     | +_+   

collapsing a QS will
   superpose itself with each of its orthogonal neighbors
   collapse to a random value
   superpose its neighbors with their orthogonal neighbors

superpose and self with orthogonal states' constraints

*/
