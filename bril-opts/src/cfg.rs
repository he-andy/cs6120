use std::collections::{HashMap, HashSet, VecDeque};

use bril_rs::Code;
use petgraph::dot::Dot;
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::fmt::Write;

use crate::utils::BasicBlock;
use crate::utils::{basic_blocks, CFGNode, CF};

pub enum Dir {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct CFG<T: CFGNode + Clone + std::fmt::Debug> {
    //cfg
    pub graph: Graph<T, bool>,

    //defpoints of each variable
    pub defs: HashMap<String, HashSet<NodeIndex>>,
}

//TODO: reverse postorder shit?
impl<T: CFGNode + Clone + std::fmt::Debug + std::fmt::Display> CFG<T> {
    pub fn new(stmts: &Vec<T>) -> Self {
        let mut graph = Graph::<T, bool>::new();

        //adds nodes to graph and builds a map of all labels
        let label_map = stmts
            .iter()
            .map(|x| {
                let idx = graph.add_node(x.clone());
                (x.is_label(), idx)
            })
            .filter_map(|x| match x.0 {
                Some(label) => Some((label, x.1)),
                None => None,
            })
            .collect::<HashMap<String, NodeIndex>>();
        println!(
            "{:?}",
            stmts.into_iter().map(|x| x.is_label()).collect::<Vec<_>>()
        );
        let mut defs = HashMap::<String, HashSet<NodeIndex>>::new();
        //Build edges + def table
        for node in graph.node_indices() {
            //build defs table
            for var in graph.node_weight(node).unwrap().defs() {
                match defs.get_mut(&var) {
                    Some(set) => {
                        set.insert(node.clone());
                    }
                    None => {
                        defs.insert(var, HashSet::from([node]));
                    }
                };
            }

            match graph.node_weight(node).unwrap().control_flow() {
                CF::Jump(label) => {
                    println!("Jump to {}, label_map: {:?}", label, label_map);
                    let _ = graph.add_edge(node, *label_map.get(&label).unwrap(), false);
                }

                CF::Branch(lt, lf) => {
                    if let Some(label) = label_map.get(&lt) {
                        graph.add_edge(node, *label, true);
                    }
                    if let Some(label) = label_map.get(&lf) {
                        graph.add_edge(node, *label, false);
                    }
                }

                CF::Normal | CF::Label(_) => {
                    let next = node.index() + 1;
                    if next < stmts.len() {
                        graph.add_edge(node, (next as u32).into(), false);
                    }
                }

                CF::Return => {
                    //do nothing
                }
            }
        }
        Self { graph, defs }
    }

    /// returns entry point of CFG, assumes that the graph is nonempty
    pub fn start(&self) -> NodeIndex {
        self.graph.node_indices().next().unwrap()
    }

    pub fn recompute_defs(&mut self) {
        let mut defs = HashMap::<String, HashSet<NodeIndex>>::new();
        for node in self.graph.node_indices() {
            //build defs table
            for var in self.graph.node_weight(node).unwrap().defs() {
                match defs.get_mut(&var) {
                    Some(set) => {
                        set.insert(node.clone());
                    }
                    None => {
                        defs.insert(var, HashSet::from([node]));
                    }
                };
            }
        }
        self.defs = defs;
    }

    pub fn def(&self, var: &String) -> HashSet<NodeIndex> {
        match self.defs.get(var) {
            Some(set) => set.clone(),
            None => HashSet::new(),
        }
    }

    pub fn reverse_postorder(&self) -> Vec<NodeIndex> {
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();
        let mut work_list = Vec::new();
        work_list.push(self.start());
        while let Some(node) = work_list.pop() {
            if !visited.contains(&node) {
                visited.insert(node);
                work_list.push(node);
                for neighbor in self.graph.neighbors(node) {
                    if !visited.contains(&neighbor) {
                        work_list.push(neighbor);
                    }
                }
            } else {
                postorder.push(node);
            }
        }
        postorder.reverse();
        postorder
    }

    pub fn work_list<U: PartialEq + Clone + std::fmt::Debug>(
        &self,
        meet: fn(&U, &U) -> U,                    //meet function
        transfer: fn(&U, &NodeIndex, &Self) -> U, //F[n]
        top: U,                                   //initial value of out[n]
        init: U,                                  //initial value of meet (usually empty set)
        direction: Dir, //direction of analysis, Dir::Forward or Dir::Backward
    ) -> (Vec<U>, Vec<U>) {
        let mut work_list = VecDeque::new();
        let mut work_list_set = HashSet::new();
        let mut out_n = vec![top.clone(); self.graph.node_count()];
        let mut in_n = vec![top.clone(); self.graph.node_count()];
        let mut visited = Vec::new();
        let direction = match direction {
            Dir::Forward => petgraph::Direction::Incoming,
            Dir::Backward => petgraph::Direction::Outgoing,
        };

        let reverse_dir = match direction {
            petgraph::Direction::Incoming => petgraph::Direction::Outgoing,
            petgraph::Direction::Outgoing => petgraph::Direction::Incoming,
        };

        for node in self.graph.node_indices() {
            work_list.push_back(node);
            work_list_set.insert(node);
        }

        while !work_list.is_empty() {
            let node = work_list.pop_front().unwrap();
            work_list_set.remove(&node);
            //meet of pred/succ nodes (depending on direction), assumes that init is empty set
            match direction {
                //Backward analysis, meet of successors
                petgraph::Direction::Outgoing => {
                    let out = self
                        .graph
                        .neighbors_directed(node, direction)
                        .fold(init.clone(), |acc, x| meet(&acc, &in_n[x.index()]));
                    let new_result = transfer(&out, &node, self);
                    out_n[node.index()] = out;
                    //if result changed, add to worklist
                    if new_result != in_n[node.index()] {
                        in_n[node.index()] = new_result;
                        visited.push(node);
                        for neighbor in self.graph.neighbors_directed(node, reverse_dir) {
                            if work_list_set.insert(neighbor) {
                                work_list.push_back(neighbor);
                            }
                        }
                    }
                }
                //Forward analysis, meet of predecessors
                petgraph::Direction::Incoming => {
                    let in_ = self
                        .graph
                        .neighbors_directed(node, direction)
                        .fold(init.clone(), |acc, x| meet(&acc, &out_n[x.index()]));
                    let new_result = transfer(&in_, &node, self);
                    in_n[node.index()] = in_;
                    //if result changed, add to worklist
                    if new_result != out_n[node.index()] {
                        out_n[node.index()] = new_result;
                        visited.push(node);
                        for neighbor in self.graph.neighbors_directed(node, reverse_dir) {
                            if work_list_set.insert(neighbor) {
                                work_list.push_back(neighbor);
                            }
                        }
                    }
                }
            };
        }

        (in_n, out_n)
    }

    pub fn delete_unreachable(&mut self) {
        let reachable = self.reachable_from_start();
        self.graph.retain_nodes(|_, node| reachable.contains(&node));
        self.recompute_defs();
    }

    pub fn reachable_from_start(&self) -> HashSet<NodeIndex> {
        let mut visited = HashSet::new();
        self.reachable_helper(&self.start(), &mut visited);
        visited
    }

    fn reachable_helper(&self, node: &NodeIndex, visited: &mut HashSet<NodeIndex>) {
        visited.insert(*node);
        for neighbor in self.graph.neighbors(*node) {
            if !visited.contains(&neighbor) {
                self.reachable_helper(&neighbor, visited);
            }
        }
    }
    pub fn debug_cfg(&self) {
        println!("{}", Dot::with_config(&self.graph, &[]));
    }

    pub fn debug_cfg_string(&self) -> String {
        let mut out = String::new();
        writeln!(out, "{:?}", Dot::with_config(&self.graph, &[]));
        out
    }
}

pub fn graph_from_bblocks(stmts: Vec<Code>) -> CFG<BasicBlock> {
    CFG::new(&basic_blocks(stmts))
}

//trace finding
impl CFG<BasicBlock> {
    /// assigns labels to unlabelled blocks
    pub fn label(&mut self) {
        let start = self.graph.node_weight_mut(self.start()).unwrap();
        if start.label.is_none() {
            start.label = Some("_CFG_ENTRY".to_string());
        }
        for node in self.graph.node_indices() {
            let bb = self.graph.node_weight_mut(node).unwrap();
            if bb.label.is_none() {
                bb.label = Some(format!("_CFG_L{}", node.index()));
            }
        }
    }

    /// performs trace analysis and block reordering to get a vector of LIRNodes
    /// returns a vector of LIRNodes and the size of the resulting LIRTree
    pub fn flatten(&mut self) -> Vec<Code> {
        let mut marked = HashSet::new();
        let mut counts = HashMap::new();
        let mut traces = vec![];

        let mut removed_edges = vec![];
        for edge in self.graph.edge_indices() {
            if *self.graph.edge_weight(edge).unwrap() {
                removed_edges.push(self.graph.edge_endpoints(edge).unwrap());
            }
        }

        for edge in &removed_edges {
            self.graph
                .remove_edge(self.graph.find_edge(edge.0, edge.1).unwrap());
        }

        loop {
            if let Some(source) = Self::heuristic(self.graph.node_indices(), &marked, &counts) {
                let mut memo = HashMap::new();
                Self::find_maximal_trace(&mut marked, &mut memo, source, &self.graph);
                let trace = Self::reconstruct_trace(&memo, source);
                Self::update_counts(&trace, &memo, &mut counts);
                for block in trace.iter() {
                    marked.insert(*block);
                }
                traces.push(trace);
            } else {
                break;
            }
        }

        let jumps_to_insert = Self::reorder_traces(&mut traces, &self.graph);
        let code = traces
            .into_iter()
            .enumerate()
            .map(|(idx, x)| {
                let mut extract_node_weights = x
                    .into_iter()
                    .map(|x| self.graph.node_weight(x).unwrap().clone())
                    .collect::<Vec<_>>();

                for i in 0..extract_node_weights.len() - 1 {
                    let x = extract_node_weights.get_mut(i).unwrap();
                    if let CF::Jump(_) = x.control_flow() {
                        x.instructions.pop();
                    }
                }

                let mut conv_lir = extract_node_weights
                    .iter()
                    .map(|x| x.as_code_block())
                    .flatten()
                    .collect::<Vec<_>>();

                if let Some(jtarget) = jumps_to_insert[idx] {
                    //if the jump is already there, no need to add
                    if let CF::Jump(_) = conv_lir.last().unwrap().control_flow() {
                        ();
                    } else {
                        let target = self
                            .graph
                            .node_weight_mut(NodeIndex::new(jtarget as usize))
                            .unwrap();
                        match target.is_label() {
                            Some(t) => {
                                conv_lir.push(Code::Instruction(bril_rs::Instruction::Effect {
                                    args: vec![],
                                    funcs: vec![],
                                    labels: vec![t],
                                    op: bril_rs::EffectOps::Jump,
                                    pos: None,
                                }))
                            }
                            None => {
                                panic!();
                            }
                        }
                    }
                } else {
                    //if there is a jump, remove it (extraneous)
                    if let CF::Jump(_) = conv_lir.last().unwrap().control_flow() {
                        conv_lir.pop();
                    }
                }
                conv_lir
            })
            .flatten()
            .collect::<Vec<Code>>();

        //remove extra labels
        // let used_labels = code
        //     .iter()
        //     .map(|x| match x.control_flow() {
        //         CF::Jump(l) => vec![l],
        //         CF::Branch(l1, l2) => vec![l1, l2],
        //         _ => vec![],
        //     })
        //     .flatten()
        //     .collect::<HashSet<_>>();
        // let mut removed_extra_labels = code;

        //restore removed edges
        for edge in removed_edges {
            self.graph.add_edge(edge.0, edge.1, true);
        }

        code
    }

    fn heuristic(
        nodes: petgraph::graph::NodeIndices,
        marked: &HashSet<NodeIndex>,
        counts: &HashMap<NodeIndex, usize>,
    ) -> Option<NodeIndex> {
        if nodes.len() == marked.len() {
            None
        } else {
            nodes.min_by_key(|x| {
                if marked.contains(x) {
                    (std::usize::MAX, 0)
                } else {
                    match counts.get(x) {
                        None => (0, x.index()),
                        Some(v) => (*v, x.index()),
                    }
                }
            })
        }
    }

    fn find_maximal_trace<T>(
        marked: &mut HashSet<NodeIndex>,
        memo: &mut HashMap<NodeIndex, (i32, Option<NodeIndex>)>,
        source: NodeIndex,
        cfg: &Graph<T, bool>,
    ) -> i32 {
        marked.insert(source);
        let entry = match cfg
            .neighbors(source)
            .map(|x| {
                if !marked.contains(&x) {
                    match memo.get(&x) {
                        Some((trace_len, _)) => (trace_len.clone(), x),
                        None => (Self::find_maximal_trace(marked, memo, x, cfg), x),
                    }
                } else {
                    (-1, x)
                }
            })
            .filter(|(v, _)| v >= &0)
            .max()
        {
            Some((trace_len, next)) => (trace_len + 1, Some(next)),
            None => (1, None),
        };
        marked.remove(&source);
        memo.insert(source, entry);
        return entry.0;
    }

    fn reconstruct_trace(
        memo: &HashMap<NodeIndex, (i32, Option<NodeIndex>)>,
        source: NodeIndex,
    ) -> Vec<NodeIndex> {
        let mut trace = vec![source];
        let mut next = memo.get(&source).unwrap().1;
        while next != None {
            trace.push(next.unwrap());
            next = memo.get(&next.unwrap()).unwrap().1;
        }
        trace
    }

    #[inline]
    fn update_counts(
        trace: &Vec<NodeIndex>,
        memo: &HashMap<NodeIndex, (i32, Option<NodeIndex>)>,
        counts: &mut HashMap<NodeIndex, usize>,
    ) {
        let l = trace.len();
        for n in memo.keys() {
            match counts.get(n) {
                Some(v) => counts.insert(*n, v + l),
                None => counts.insert(*n, l),
            };
        }
    }

    fn reorder_traces<T>(
        trace_list: &mut Vec<Vec<NodeIndex>>,
        graph: &Graph<T, bool>,
    ) -> Vec<Option<isize>> {
        //move first trace to start
        let start = (0..trace_list.len())
            .min_by_key(|x| trace_list[*x].first().unwrap().index())
            .unwrap();
        trace_list.swap(0, start);

        let mut trace_starting_with = trace_list
            .iter()
            .enumerate()
            .map(|(idx, x)| (x.first().unwrap().index(), idx))
            .collect::<HashMap<_, _>>();

        let mut inserted_jumps = vec![None; trace_list.len()];
        for i in 0..trace_list.len() {
            let last = trace_list[i].last().unwrap().index();
            let next = graph.edges(NodeIndex::new(last)).next();
            match next {
                Some(edge) => {
                    assert!(*edge.weight() == false);
                    let target = edge.target().index();
                    let target_idx = trace_starting_with.get(&target);
                    match target_idx {
                        Some(idx) => {
                            if *idx > i {
                                trace_list.swap(i, *idx);
                                trace_starting_with
                                    .insert(trace_list[*idx].first().unwrap().index(), *idx);
                            } else {
                                inserted_jumps[i] = Some(target as isize);
                            }
                        }
                        None => {
                            inserted_jumps[i] = Some(target as isize);
                        }
                    }
                }
                None => inserted_jumps[i] = None,
            }
        }
        inserted_jumps
    }
}
