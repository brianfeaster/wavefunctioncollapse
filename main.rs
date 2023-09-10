#![allow(non_snake_case)]

use std::{
    collections::HashSet,
    env,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    iter::FromIterator,
    io::stdin
};


type Res<T> = Result<T, Box<dyn Error>>;

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


#[derive(Debug)]
struct State {
    id: usize,
    glyph: String,
    projections: Vec<SuperState>
}

impl State {
    fn new (id: usize, glyph: &str, projections: &[&[usize]]) -> State {
        State{
            id,
            glyph: glyph.to_string(),
            projections: projections.iter()
                .map(|ss| SuperState::from(ss))
                .collect::<Vec<SuperState>>()
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
    fn project (&self, hss: &HashSet<usize>) -> HashSet<usize> {
        HashSet::from_iter(self.states.intersection(hss).map(|i|*i))
    }
    fn count (&self) -> usize {
        self.states.len()
    }
    fn states (&self) -> impl Iterator<Item=&usize> + '_ {
        self.states.iter()
    }
    fn collapse (&mut self) -> usize {
        let i = *self.states.iter()
            .next().ok_or("superstate empty")
            .map_err(|e| println!("ERROR: {:?}", e))
            .unwrap();
        self.states.clear();
        self.states.insert(i);
        i
    }
}

impl Debug for SuperState {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str(&format!("{:?}", self.states));
        Ok(())
    }
}
////////////////////////////////////////

struct WaveFunction {
  term: Term,
  states: Vec<State>,
  grid: Vec<Vec<SuperState>>, // Grid of states (state == one or more possible values)
  groups: Vec<HashSet<(usize,usize)>> // Group values by wave count
}

impl WaveFunction {
    fn new (states: Vec<State>, ss: &[usize]) -> WaveFunction {
        let mut groups: Vec<HashSet<(usize, usize)>> = (0..=4).into_iter().map(|_| HashSet::new()).collect();
        let term = Term::new();
        WaveFunction{
            grid: (0..term.h).into_iter()
                .map(|y| (0..term.w).into_iter()
                    .map(|x| {
                        groups[4].insert((y,x));
                        SuperState::from(ss)
                    }).collect())
                .collect(),
            term, states,
            groups
        }
    }
    //fn projection(&self, id: usize, dir: usize) -> &SuperState {
    //    &self.states[id].projections[dir]
    //}
    fn projectdir(&mut self, y: usize, x: usize, oy: usize, ox: usize, dir: usize) {


        let sscount = self.grid[y][x].count(); // trying to project onto collapsed state
        if sscount < 2 { return }

        let mut projected_super_state = HashSet::new(); // Assemble projection sstate
        let id = self.grid[oy][ox].states().for_each( |id|
            self.states[*id].projections[dir].states.iter().for_each(|id| {
                projected_super_state.insert(*id);
            }));


        let hashset2 = self.grid[y][x].project(&projected_super_state);
        let ss = &mut self.grid[y][x];
        ss.states.clear();
        ss.states = hashset2;

        let sscountfinal = ss.states.len();

      //println!("{:?}", self);
      //if 0 == sscountfinal{ println!("\x1b[31mERROR-IS-ZERO-{},{}",y,x); }
      //let mut buff = String::new();
      //println!("{:?}", stdin().read_line(&mut buff));


        if sscount != sscountfinal {
            let p = (y, x);
            self.groups[sscount].remove(&p);
            self.groups[sscountfinal].insert(p);
            //if 1 == sscountfinal { self.projectState(y, x) }
            if 0 != sscountfinal { self.projectState(y, x) }
        }
    }
    fn projectState(&mut self, y: usize, x: usize) {
        self.projectdir((y+self.term.h-1)%self.term.h, x, y,x, 0);
        self.projectdir((y+1)            %self.term.h, x, y,x, 1);
        self.projectdir(y, (x+1)            %self.term.w, y,x, 2);
        self.projectdir(y, (x+self.term.w-1)%self.term.w, y,x, 3);
    }
    fn collapse(&mut self, y: usize, x: usize) {
        let p = (y, x);
        // Move state to 1-state group
        let sscount = self.grid[y][x].count();
        self.groups[sscount].remove(&p);
        self.groups[1].insert(p);
        self.grid[y][x].collapse();
        if 0 == self.grid[y][x].count() { println!("\x1b[31mERROR-IS-ZERO-{},{}",y,x); }
        self.projectState(y, x);
    }
    fn collapseMaybe(&mut self) -> bool {
        let (y,x) = if 0 < self.groups[2].len() {
            let (y,x) = { let p = self.groups[2].iter().next().unwrap(); (p.0, p.1) };
            self.groups[2].remove(&(y, x));
            (y, x)
        } else if 0 < self.groups[3].len() {
            let (y,x) = { let p = self.groups[3].iter().next().unwrap(); (p.0, p.1) };
            self.groups[3].remove(&(y, x));
            (y, x)
        } else if 0 < self.groups[4].len() {
            let (y,x) = { let p = self.groups[4].iter().next().unwrap(); (p.0, p.1) };
            self.groups[4].remove(&(y, x));
            (y, x)
        } else {
            return false
       };
       self.collapse(y, x);
       true
    }
}

impl Debug for WaveFunction {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        //fmt.write_str("\n");
        self.grid.iter().for_each(|r| {
            r.iter().for_each(|ss| {
                    let mut s = 0;
                    fmt.write_str( &"\x1b[100m" );
                    fmt.write_str( if ss.states.get(&0).is_some() { &" " } else { s+=1; &"" } );
                    fmt.write_str( if ss.states.get(&1).is_some() { &"+" } else { s+=1; &"" } );
                    fmt.write_str( if ss.states.get(&2).is_some() { &"-" } else { s+=1; &"" } );
                    fmt.write_str( if ss.states.get(&3).is_some() { &"|" } else { s+=1; &"" } );
                    fmt.write_str( &"\x1b[0m " );
                    fmt.write_str( &"    "[0..s] );
            });
            fmt.write_str("\n");
        });
        fmt.write_str(&format!("{:?}\n", self.groups[0]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[1]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[2]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[3]))?;
        fmt.write_str(&format!("{:?}\n", self.groups[4]))?;
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
                };
            });
            fmt.write_str("\n");
        });
        Ok(())
    }
}


// Main //////////////////////////////////////////////////////////////

fn header () {
  println!("\x1b[31m__        _______ ____ ");
  println!("\x1b[33m\\ \\      / /  ___/ ___|");
  println!("\x1b[32m \\ \\ /\\ / /| |_ | |    ");
  println!("\x1b[34m  \\ V  V / |  _|| |___ ");
  println!("\x1b[35m   \\_/\\_/  |_|   \\____|\x1b[0m");
}

pub fn main () {
  //header();
  let states = vec!( //  D A/B C
    State::new(0, " ", &[&[0,1,2  ],&[0,1,2  ],&[0,1,  3],&[0,1,  3]]),
    //State::new(1, "+", &[&[0,1,  3],&[0,1,  3],&[0,1,2  ],&[0,1,2  ]]),
    State::new(1, "+", &[&[0,    3],&[0,    3],&[0,  2  ],&[0,  2  ]]), // no ++ connections
    State::new(2, "-", &[&[0,  2  ],&[0,  2  ],&[  1,2  ],&[  1,2  ]]),
    State::new(3, "|", &[&[  1,  3],&[  1,  3],&[0,    3],&[0,    3]]),
  );

  //for s in &states { println!("{:?}", s); }

  let mut wf = WaveFunction::new(states, &[0,1,2,3]);

  //let x=0; let y=0; wf.collapse(y,x);
  print!("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n", (wf.term.w+1)*wf.term.h);
  while wf.collapseMaybe() {
      //let mut buff = String::new();
      //println!("{:?}", stdin().read_line(&mut buff));
  }
      print!("{}", wf);
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
