#![allow(non_snake_case)]

use std::{
    collections::HashSet,
    env,
    error::Error,
    fmt::{self, Debug, Formatter},
    iter::FromIterator,
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

// Util //////////////////////////////////////////////////////////////

fn bitscount (w: usize) -> usize {
  let mut c = ((w & 0xaaaa)>>1) + (w&0x5555);   
  c = ((c & 0xcccc)>>2) + (c&0x3333);
  c = ((c & 0xf0f0)>>4) + (c&0x0f0f);
  ((c & 0xff00)>>8) + (c&0x00ff)
}

fn rndbit (mut word: usize) -> Option<usize> {
  let mut hs = HashSet::new();
  let mut bit = 0;
  while 0 < word {
    if 1 == (word & 1) { hs.insert(bit); }
    bit += 1;
    word = word>>1;
  }
  hs.iter().next().copied()
}

// Qstate ////////////////////////////////////////////////////////////

/*
  *  [ -]  *        [[ |]+[+-]]
[+-]  [-] [+-]     *[[+|]+[ -]] = [|] + [ ] + [+] + [-]
  *  [ -]  *

     a[-]b[+-]      [  [+-]   ]               collapsing a superposes b
[+-]  [-] [+-]     *[[+|]+[ -]] = [+] + [-]
     [ -]
____________________

Collapse center
Superpose orthogonals

         +-|
  +-|    +      +-|
         +-|

              
   +     +-+ |
   |     | +_+   

collapsing a QS will
   superpose itself with each of its orthogonal neighbors
   collapse to a random value
   superpose its neighbors with their orthogonal neighbors


superpose
  and self with orthogonal states' constraints

 0 1 2 3  values
   + - |  bits qs=15
*/



////////////////////////////////////////

//  D A C
//    B

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
    fn project (&mut self, ss: &SuperState) {
        let ss2 = HashSet::from_iter(self.states.intersection(&ss.states).map(|i|*i));
        self.states.clear();
        self.states = ss2;
    }
    fn collapse (&mut self) {
        let i = *self.states.iter()
            .next().ok_or("superstate empty")
            .map_err(|e| println!("ERROR: {:?}", e))
            .unwrap();
        self.states.clear();
        self.states.insert(i);
    }
}

impl Debug for SuperState {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str(&format!("{:?}", self.states));
        Ok(())
    }
}
////////////////////////////////////////

#[derive(Debug)]
struct Qstate { // Qstate
  values: Vec<char>,
  rules:  Vec<Vec<usize>> // CONSTRAINTS value<direction<mask>>
}

impl Qstate {
  fn new (values: &[char]) -> Qstate {
    Qstate{
      values: Vec::from(values),
      rules:  values.iter()
                .map( |_| vec![(1<<4)-1; values.len()] ) // all bits for each value set
                .collect::<Vec<Vec<usize>>>()
    }
  }

  fn count (&self) -> usize { self.values.len() }

  // Specify allowed values in each direction for specified value
  // Represented internally as a bitfield.
  fn mask (&mut self, value: usize, dirmasks: &[&[usize]]) {
    self.rules[value].iter_mut()
      .zip(dirmasks.iter())
      .for_each( |(n, d)|
          *n = d.iter()
                .fold(0, |r,p| r+(1<<*p)));
  }
}

// States ////////////////////////////////////////////////////////////

#[derive(Debug)]
struct States {
  w: usize,
  h: usize,
  field: Vec<usize>, // Grid of states (state == one or more possible values)
  groups: Vec<HashSet<usize>> // Group values by wave count
}

impl States {
  fn new (w: usize, h: usize, count: usize) -> States {
    States{
      w, h,
      field:  vec![(1<<count)-1; w*h],
      groups: (0..count)
                .map( |i|
                  if i==count-1 { (0..w*h).collect::<HashSet<usize>>() } 
                  else { HashSet::new() } )
                .collect()
    }
  }
  // pick random state with fewest uncollapsed values
  fn smallestStateGroup (&mut self) -> Option<&mut HashSet<usize>> {
    println!("lenghts {} {} {} {}",
      self.groups[0].len(),
      self.groups[1].len(),
      self.groups[2].len(),
      self.groups[3].len());
    self.groups.iter_mut()
      .skip(1) // Skip groups with single state values.
      .filter( |group| 0<group.len() )
      .next()
  }
  //fn collapse () // Force eigenstate to single state.  Must be eigenstate with > 2 states into a single state.
  //fn superpose () // Add this and specified waves to each other.  Neither should end up empty.
  fn nextCollapse (&mut self, values: &Qstate) -> Option<bool> {
    let group = self.smallestStateGroup()?;

    // consider random state
    let idx: usize = *group.iter().next().unwrap();

    // move state to collapsed set
    group.remove(&idx);
    self.groups[0].insert(idx);

    let (x,y) = (idx % self.w, idx / self.w);
    let up =   x                   + ((self.h+y-1)%self.h)*self.w;
    let down = x                   + ((       y+1)%self.h)*self.w;
    let left = (self.w+x-1)%self.w + y                    *self.w;
    let right= (       x+1)%self.w + y                    *self.w;
    println!("@{} x{} y{}  up{} down{} left{} rigt{}", idx, x, y, up, down, left, right);

    // consider random state
    println!("states @{}  u{} d{} l{} r{}", self.field[idx], self.field[up], self.field[down], self.field[left], self.field[right]);
    let state = self.field[idx] & self.field[up] & self.field[down] & self.field[left] & self.field[right];
    //let isAlreadyOne = 1 == bitscount(state);

    // Collapse state to single value

    let ornd = rndbit(state); // 0..3
    //println!("idx={}  state={} =>  ornd={:?} {}", idx, state, ornd, isAlreadyOne);
    let rnd = match ornd {
      Some(idx) => idx,
      None => return Some(false)
    };

    let value = 1<<rnd;       // eigenvalue
    self.field[idx] = value;
    println!("collapsing {} to {:?}", state, value);

    //if isAlreadyOne { return Some(true) }

    // Add collapsed state to its neighbors (lowering their entropy)

    let value = &values.values[rnd];
    let rules = &values.rules[rnd];
    println!("collapsing {},{};{:<2} {:08b} -> {} {} rules{:?}",
      x,y, idx, state, rnd, value, rules);

    let upBitCount = bitscount(self.field[up]);
    let downBitCount = bitscount(self.field[down]);
    let leftBitCount = bitscount(self.field[left]);
    let rightBitCount = bitscount(self.field[right]);

    if upBitCount < 2 { println!("error up={} Bitcount {}", up, upBitCount); }
    if downBitCount < 2 { println!("error dn={} Bitcount {}", down, downBitCount); }
    if leftBitCount < 2 { println!("error lf={} Bitcount {}", left, leftBitCount); }
    if rightBitCount < 2 { println!("error ri={} Bitcount {}", right, rightBitCount); }

    if 1 < bitscount(self.field[up]) { self.field[up] &= values.rules[rnd][0] }
    if 1 < bitscount(self.field[down]) { self.field[down] &= values.rules[rnd][2] }
    if 1 < bitscount(self.field[left]) { self.field[left] &= values.rules[rnd][3] }
    if 1 < bitscount(self.field[right]) { self.field[right] &= values.rules[rnd][1] }

    let upBitCountNew = bitscount(self.field[up]);
    let downBitCountNew = bitscount(self.field[down]);
    let leftBitCountNew = bitscount(self.field[left]);
    let rightBitCountNew = bitscount(self.field[right]);

    println!("{}->{}  {}->{}  {}->{}  {}->{}",
      upBitCount, upBitCountNew,
      downBitCount, downBitCountNew,
      leftBitCount, leftBitCountNew,
      rightBitCount, rightBitCountNew);
    if upBitCount != upBitCountNew       { self.groups[upBitCount-1].remove(&up); self.groups[upBitCountNew-1].insert(up); }
    if downBitCount != downBitCountNew   { self.groups[downBitCount-1].remove(&down); self.groups[downBitCountNew-1].insert(down); }
    if leftBitCount != leftBitCountNew   { self.groups[leftBitCount-1].remove(&left); self.groups[leftBitCountNew-1].insert(left); }
    if rightBitCount != rightBitCountNew { self.groups[rightBitCount-1].remove(&right); self.groups[rightBitCountNew-1].insert(right); }

    Some(true)
  }
}

// Main //////////////////////////////////////////////////////////////

pub fn main () {
  let term = Term::new(); //let term = Term{w:8, h:8};
  let mut states = vec!( //  D A/B C
    State::new(0, " ", &[&[0,1,2  ],&[0,1,2  ],&[0,1,  3],&[0,1,  3]]),
    State::new(1, "+", &[&[0,1,  3],&[0,1,  3],&[0,1,2  ],&[0,1,2  ]]),
    State::new(2, "-", &[&[0,  2  ],&[0,  2  ],&[  1,2  ],&[  1,2  ]]),
    State::new(3, "|", &[&[  1,  3],&[  1,  3],&[0,    3],&[0,    3]]),
  );
  println!("\x1b[31m__        _______ ____ ");
  println!("\x1b[33m\\ \\      / /  ___/ ___|");
  println!("\x1b[32m \\ \\ /\\ / /| |_ | |    ");
  println!("\x1b[34m  \\ V  V / |  _|| |___ ");
  println!("\x1b[35m   \\_/\\_/  |_|   \\____|\x1b[0m");

  println!("{:?}", term);

  for s in &states {
    println!("{:?}", s);
  }

  for s in &mut states {
    s.projections[0].collapse();
  }
  println!("");

  for s in &states {
    println!("{:?}", s);
  }

  return;
  
  let mut values = Qstate::new(&[' ','+', '-', '|']);
  //                up       right    down     left
  values.mask(0, &[&[0,2], &[0,3], &[0,2], &[0,3]]);
  values.mask(1, &[&[1,3], &[1,2], &[1,3], &[1,2]]);
  values.mask(2, &[&[0,2], &[1,2], &[0,2], &[1,2]]);
  values.mask(3, &[&[1,3], &[0,3], &[1,3], &[0,3]]);
  let mut states = States::new(term.w, term.h, values.count());
  //println!("{:?}\n\n{:?}\n\n{:?}", term, values, states);
  while states.nextCollapse(&values).is_some() {
    //println!("{:?}\n\n{:?}\n\n{:?}", term, values, states);
    for y in 0..term.h {
      for x in 0..term.w {
        let state = states.field[x+y*term.w];
        let bc = bitscount(state);
        if 1 == bc {
          let rb = rndbit(state).unwrap();
          print!("{}", values.values[rb])
        } else {
          print!("{}", bc)
        }
      }
      println!()
    }
    let mut buff = String::new();
    println!("{:?}", std::io::stdin().read_line(&mut buff));
  }
}


/*
 * https://en.wikipedia.org/wiki/Quantum_superposition
 * https://en.wikipedia.org/wiki/Wave_function_collapse

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
*/
