#![allow(non_snake_case)]

use std::{
    collections::HashSet,
    env,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    iter::FromIterator,
    io::stdin
};


pub type Res<T> = Result<T, Box<dyn Error>>;

pub fn readline () -> String {
    let mut buff = String::new();
    stdin().read_line(&mut buff).ok();
    buff
}

// Terminal //////////////////////////////////////////////////////////

#[derive(Debug)]
struct Term {
    w:usize,
    h:usize
}

impl Term {
    fn new() -> Term {
        let mut args = env::args().skip(1).take(2).flat_map(|s| s.parse::<usize>());
        Term{
            w: args.next().unwrap_or(80),
            h: args.next().unwrap_or(25)
        }
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
    fn new (id: usize, glyph: &str, projections: &[&[usize]]) -> State {
        State{
            id,
            glyph: glyph.to_string(),
            projections: projections.iter()
                .map(|states| SuperState::from(states))
                .collect()
        }
    }
}

////////////////////////////////////////

struct SuperState {
    states: HashSet<usize>
}

impl SuperState {
    fn from (states: &[usize]) -> SuperState {
        SuperState{states:HashSet::from_iter(states.iter().map(|i|*i))}
    }
    fn intersect (&self, hss: &HashSet<usize>) -> HashSet<usize> {
        self.states.intersection(hss).map(|&i|i).collect()
    }
    fn count (&self) -> usize {
        self.states.len()
    }
    fn states (&self) -> impl Iterator<Item=usize> + '_{
        self.states.iter().map(|i|*i)
    }
    fn collapse (&mut self) -> usize {
        let i = *self.states.iter().next().expect("superstate empty");
        self.states.clear();
        self.states.insert(i);
        i
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
    states: Vec<State>,
    grid: Vec<Vec<SuperState>>, // Grid of states (state == one or more possible values)
    groups: Vec<HashSet<(usize,usize)>> // Group values by wave count
}

impl WaveFunction {
    fn new (states: Vec<State>) -> WaveFunction {
        let term = Term::new();
        let numStates = states.len();
        let fullSuperState = &(0..numStates).collect::<Vec<_>>()[..];
        let mut groups: Vec<HashSet<_>> = (0..=numStates)
            .into_iter()
            .map(|_| HashSet::new())
            .collect();
        WaveFunction{
            grid: (0..term.h).into_iter()
                .map(|y| (0..term.w).into_iter()
                    .map(|x| {
                        groups[numStates].insert((y,x));
                        SuperState::from(fullSuperState)
                    }).collect())
                .collect(),
            term, states,
            groups
        }
    }
    fn projectdir(&mut self, p: (usize, usize), oy: usize, ox: usize, dir: usize) {
        let (y, x) = p;
        let sscount = self.grid[y][x].count();
        if sscount < 2 { return } // Skip collapsed state

        let mut projected_super_state = HashSet::new(); // Assemble projection sstate
        self.grid[oy][ox].states().for_each( |id| // over all states in this superstate...
            self.states[id].projections[dir].states.iter().for_each(|id| { // union states in this direction creating a projection
                projected_super_state.insert(*id);
            }));

        let hashset2 = self.grid[y][x].intersect(&projected_super_state);
        let ss = &mut self.grid[y][x];
        ss.states.clear();
        ss.states = hashset2;

        let sscountfinal = ss.states.len();

        if sscount != sscountfinal {
            self.groups[sscount].remove(&p);
            self.groups[sscountfinal].insert(p);
            self.projectState(p)
        }
    }
    fn projectState(&mut self, p: (usize, usize)) {
        let (y, x) = p;
        self.projectdir(((y+self.term.h-1)%self.term.h, x), y,x, 0);
        self.projectdir(((y+1)            %self.term.h, x), y,x, 1);
        self.projectdir((y, (x+1)            %self.term.w), y,x, 2);
        self.projectdir((y, (x+self.term.w-1)%self.term.w), y,x, 3);
    }
    fn collapseAt(&mut self, p: (usize, usize)) {
        let (y, x) = p;
        assert!(2 <= self.grid[y][x].count()); // Should only collapse superstates
        self.grid[y][x].collapse();
        self.projectState(p);
    }
    fn getLowestEntropy(&mut self) -> Option<(usize, usize)> {
        self.groups.iter_mut()
            .skip(2)
            .find(|v| 0<v.len())
            .map(|v| v
                .take(&v.iter().next().unwrap().clone())
                .unwrap())
            .map(|p| { // Move to 1-state group
                self.groups[1].insert(p);
                p
            })
    }
    pub fn collapseMaybe(&mut self) -> bool {
        match self.getLowestEntropy() {
            Some(p) => { self.collapseAt(p); true}
            None => false
        }
    }
    pub fn print (&self) { print!("{}\x1b[0m", self) }
    pub fn debug (&self) { print!("{:?}\x1b[0m", self) }
}

impl Debug for WaveFunction {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.grid.iter().for_each(|r| {
            r.iter().for_each(|ss| {
                let mut s = 0;
                fmt.write_str(&"\x1b[100m" ).ok();
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
        fmt.write_str(&format!("{:?}\n", self.groups[0]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[1]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[2]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[3]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[4]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[5]))?;
        Ok(())
    }
}

impl Display for WaveFunction {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.grid.iter().for_each(|r| {
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

fn moneyDungeon () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0, " ", &[&[0,1,2  ,4],&[0,1,2  ,4],&[0,1,  3,4],&[0,1,  3,4]]),
        //State::new(1, "+", &[&[0,1,  3],&[0,1,  3],&[0,1,2  ],&[0,1,2  ]]),
        State::new(1, "\x1b[1;31m+", &[&[0,    3],&[0,    3],&[0,  2  ],&[0,  2  ]]), // no ++ connections
        State::new(2, "\x1b[1;31m-", &[&[0      ],&[0      ],&[  1,2  ],&[  1,2  ]]),
        State::new(3, "\x1b[1;31m|", &[&[  1,  3],&[  1,  3],&[0      ],&[0      ]]),
        State::new(4, "\x1b[1;32m$", &[&[0,     ],&[0,     ],&[0,     ],&[0,     ]]),
    ));
    while wf.collapseMaybe() {
        if !true { print!("{}", wf); readline(); }
    }
    wf
}

pub fn ultima () -> WaveFunction {
    let mut wf = WaveFunction::new(vec!(
        State::new(0, "\x1b[44m ", &[&[0,1],  &[0,1],  &[0,1],  &[0,1]]),
        State::new(1, "\x1b[104m ",&[&[0,1,2],&[0,1,2],&[0,1,2],&[0,1,2]]),
        State::new(2, "\x1b[43m ", &[&[1,2,3],&[1,2,3],&[1,2,3],&[1,2,3]]),
        State::new(3, "\x1b[42m ", &[&[2,3,4],&[2,3,4],&[2,3,4],&[2,3,4]]),
        State::new(4, "\x1b[47m ", &[&[3,4,5],&[3,4,5],&[3,4,5],&[3,4,5]]),
        State::new(5, "\x1b[107m ",&[&[4,5],  &[4,5],  &[4,5],  &[4,5]]),
    ));
    while wf.collapseMaybe() {
        if !true { print!("{}", wf); readline(); }
    }
    wf
}

fn main () {
    //header();
    //print!("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n", (wf.term.w+1)*wf.term.h);
    ultima().print();
    moneyDungeon().print();
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
