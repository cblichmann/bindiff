use crate::graph::{CallGraph, FlowGraph};
use crate::fixed_points::FixedPoints;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Result, Context, bail};

pub struct DatabaseWriter {
    conn: Connection,
    path: PathBuf,
}

impl DatabaseWriter {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        if path_buf.exists() {
            std::fs::remove_file(&path_buf)
                .with_context(|| format!("Failed to remove old database: {}", path_buf.display()))?;
        }

        let conn = Connection::open(&path_buf)
            .with_context(|| format!("Failed to open SQLite database: {}", path_buf.display()))?;

        let writer = Self { conn, path: path_buf };
        writer.prepare_database().context("Failed to prepare database schema")?;
        Ok(writer)
    }

    fn prepare_database(&self) -> Result<()> {
        self.conn.execute_batch("
            DROP TABLE IF EXISTS metadata;
            DROP TABLE IF EXISTS file;
            DROP TABLE IF EXISTS instruction;
            DROP TABLE IF EXISTS basicblock;
            DROP TABLE IF EXISTS basicblockalgorithm;
            DROP TABLE IF EXISTS function;
            DROP TABLE IF EXISTS functionalgorithm;

            CREATE TABLE basicblockalgorithm (
                id SMALLINT PRIMARY KEY,
                name TEXT
            );
            CREATE TABLE functionalgorithm (
                id SMALLINT PRIMARY KEY,
                name TEXT
            );
            CREATE TABLE file (
                id INT PRIMARY KEY,
                filename TEXT,
                exefilename TEXT,
                hash CHARACTER(40),
                functions INT,
                libfunctions INT,
                calls INT,
                basicblocks INT,
                libbasicblocks INT,
                edges INT,
                libedges INT,
                instructions INT,
                libinstructions INT
            );
            CREATE TABLE metadata (
                version TEXT,
                file1 INT,
                file2 INT,
                description TEXT,
                created DATE,
                modified DATE,
                similarity DOUBLE PRECISION,
                confidence DOUBLE PRECISION,
                FOREIGN KEY(file1) REFERENCES file(id),
                FOREIGN KEY(file2) REFERENCES file(id)
            );
            CREATE TABLE function (
                id INT,
                address1 BIGINT,
                name1 TEXT,
                address2 BIGINT,
                name2 TEXT,
                similarity DOUBLE PRECISION,
                confidence DOUBLE PRECISION,
                flags INTEGER,
                algorithm SMALLINT,
                evaluate BOOLEAN,
                commentsported BOOLEAN,
                basicblocks INTEGER,
                edges INTEGER,
                instructions INTEGER,
                UNIQUE(address1, address2),
                PRIMARY KEY(id),
                FOREIGN KEY(algorithm) REFERENCES functionalgorithm(id)
            );
            CREATE TABLE basicblock (
                id INT,
                functionid INT,
                address1 BIGINT,
                address2 BIGINT,
                algorithm SMALLINT,
                evaluate BOOLEAN,
                PRIMARY KEY(id),
                FOREIGN KEY(functionid) REFERENCES function(id),
                FOREIGN KEY(algorithm) REFERENCES basicblockalgorithm(id)
            );
            CREATE TABLE instruction (
                basicblockid INT,
                address1 BIGINT,
                address2 BIGINT,
                FOREIGN KEY(basicblockid) REFERENCES basicblock(id)
            );
        ").context("Failed to execute schema creation batch")?;
        Ok(())
    }

    pub fn write(
        &mut self,
        call_graph1: &CallGraph,
        call_graph2: &CallGraph,
        flow_graphs1: &[FlowGraph],
        flow_graphs2: &[FlowGraph],
        fixed_points: &FixedPoints,
    ) -> Result<()> {
        let db_dir = self.path.parent().unwrap_or(Path::new("."));
        let prim_dir = Path::new(&call_graph1.filename).parent().unwrap_or(Path::new("."));
        let sec_dir = Path::new(&call_graph2.filename).parent().unwrap_or(Path::new("."));

        let db_abs = std::path::absolute(db_dir).context("Failed to resolve absolute path for DB dir")?;
        let prim_abs = std::path::absolute(prim_dir).context("Failed to resolve absolute path for primary dir")?;
        let sec_abs = std::path::absolute(sec_dir).context("Failed to resolve absolute path for secondary dir")?;

        if db_abs != prim_abs || db_abs != sec_abs {
            bail!(
                "Enforce same directory: .BinExport files must be in the same directory as the .BinDiff database file.\n\
                 Database dir: {}\n\
                 Primary dir: {}\n\
                 Secondary dir: {}",
                db_abs.display(),
                prim_abs.display(),
                sec_abs.display()
            );
        }

        let tx = self.conn.transaction().context("Failed to begin transaction")?;

        Self::write_metadata(&tx, call_graph1, call_graph2, flow_graphs1, flow_graphs2, fixed_points)?;
        let (func_algos, bb_algos) = Self::write_algorithms(&tx)?;
        Self::write_matches(&tx, fixed_points, &func_algos, &bb_algos, call_graph1, call_graph2, flow_graphs1, flow_graphs2)?;

        tx.commit().context("Failed to commit transaction")?;
        Ok(())
    }

    fn write_metadata(
        tx: &Connection,
        call_graph1: &CallGraph,
        call_graph2: &CallGraph,
        flow_graphs1: &[FlowGraph],
        flow_graphs2: &[FlowGraph],
        fixed_points: &FixedPoints,
    ) -> Result<()> {
        let file1 = 1;
        let file2 = 2;

        let count_stats = |fgs: &[FlowGraph]| {
            let mut bbs = 0;
            let mut edges = 0;
            let mut insts = 0;
            for fg in fgs {
                bbs += fg.graph.node_count();
                edges += fg.graph.edge_count();
                insts += fg.instructions.len();
            }
            (bbs, edges, insts)
        };

        let (bbs1, edges1, insts1) = count_stats(flow_graphs1);
        let (bbs2, edges2, insts2) = count_stats(flow_graphs2);

        let filename1 = get_clean_filename(&call_graph1.filename);
        let filename2 = get_clean_filename(&call_graph2.filename);

        tx.execute(
            "INSERT INTO file VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                file1,
                filename1,
                call_graph1.exe_filename,
                call_graph1.exe_hash,
                flow_graphs1.len() as i32,
                0,
                call_graph1.graph.edge_count() as i32,
                bbs1 as i32,
                0,
                edges1 as i32,
                0,
                insts1 as i32,
                0,
            ],
        ).context("Failed to insert file1 metadata")?;

        tx.execute(
            "INSERT INTO file VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                file2,
                filename2,
                call_graph2.exe_filename,
                call_graph2.exe_hash,
                flow_graphs2.len() as i32,
                0,
                call_graph2.graph.edge_count() as i32,
                bbs2 as i32,
                0,
                edges2 as i32,
                0,
                insts2 as i32,
                0,
            ],
        ).context("Failed to insert file2 metadata")?;

        let similarity = if !flow_graphs1.is_empty() {
            fixed_points.len() as f64 / flow_graphs1.len() as f64
        } else {
            0.0
        };
        let confidence = similarity;

        tx.execute(
            "INSERT INTO metadata VALUES (?1, ?2, ?3, ?4, DATETIME('NOW'), DATETIME('NOW'), ?5, ?6)",
            params![
                format!("BinDiff Rust Port"),
                file1,
                file2,
                "",
                similarity,
                confidence,
            ],
        ).context("Failed to insert metadata")?;

        Ok(())
    }

    fn write_algorithms(tx: &Connection) -> Result<(HashMap<String, i16>, HashMap<String, i16>)> {
        let mut func_algos = HashMap::new();
        let mut bb_algos = HashMap::new();

        let func_steps = vec![
            "function: name hash matching",
            "function: hash matching",
            "function: prime signature matching",
            "function: edges flowgraph MD index",
            "function: MD index matching (flowgraph MD index, top down)",
            "function: MD index matching (flowgraph MD index, bottom up)",
            "function: edges callgraph MD index",
            "function: edges proximity MD index",
            "function: MD index matching (callGraph MD index, top down)",
            "function: MD index matching (callGraph MD index, bottom up)",
            "function: relaxed MD index matching",
            "function: instruction count",
            "function: loop count matching",
            "function: call sequence matching(exact)",
            "function: call sequence matching(topology)",
            "function: call sequence matching(sequence)",
            "function: address sequence",
            "function: manual",
        ];
        for (i, step) in func_steps.iter().enumerate() {
            let id = (i + 1) as i16;
            tx.execute("INSERT INTO functionalgorithm VALUES (?1, ?2)", params![id, step])
                .context("Failed to insert functionalgorithm")?;
            func_algos.insert(step.to_string(), id);
        }

        let bb_steps = vec![
            "basicBlock: prime matching (4 instructions minimum)",
            "basicBlock: prime matching (0 instructions minimum)",
            "basicBlock: hash matching",
            "basicBlock: MD index matching (top down)",
            "basicBlock: MD index matching (bottom up)",
            "basicBlock: relaxed MD index matching",
            "basicBlock: call reference matching",
            "basicBlock: self loop matching",
            "basicBlock: instruction count matching",
            "basicBlock: propagation (size==1)",
        ];
        for (i, step) in bb_steps.iter().enumerate() {
            let id = (i + 1) as i16;
            tx.execute("INSERT INTO basicblockalgorithm VALUES (?1, ?2)", params![id, step])
                .context("Failed to insert basicblockalgorithm")?;
            bb_algos.insert(step.to_string(), id);
        }

        Ok((func_algos, bb_algos))
    }

    fn write_matches(
        tx: &Connection,
        fixed_points: &FixedPoints,
        func_algos: &HashMap<String, i16>,
        bb_algos: &HashMap<String, i16>,
        call_graph1: &CallGraph,
        call_graph2: &CallGraph,
        flow_graphs1: &[FlowGraph],
        flow_graphs2: &[FlowGraph],
    ) -> Result<()> {
        let mut function_id = 1;
        let mut basic_block_id = 1;

        for fp in fixed_points {
            let algo_id = func_algos.get(&fp.matching_step).cloned().unwrap_or(0);
            
            let prim_fg = flow_graphs1.iter().find(|fg| fg.entry_point_address == fp.primary_address).unwrap();
            let sec_fg = flow_graphs2.iter().find(|fg| fg.entry_point_address == fp.secondary_address).unwrap();

            let mut name1 = format!("sub_{:X}", fp.primary_address);
            let mut name2 = format!("sub_{:X}", fp.secondary_address);

            if let Some(node_idx) = call_graph1.get_vertex(fp.primary_address) {
                let name = &call_graph1.graph[node_idx].name;
                if !name.is_empty() {
                    name1 = name.clone();
                }
            }
            if let Some(node_idx) = call_graph2.get_vertex(fp.secondary_address) {
                let name = &call_graph2.graph[node_idx].name;
                if !name.is_empty() {
                    name2 = name.clone();
                }
            }

            tx.execute(
                "INSERT INTO function VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    function_id,
                    fp.primary_address as i64,
                    name1,
                    fp.secondary_address as i64,
                    name2,
                    fp.similarity,
                    fp.confidence,
                    fp.flags,
                    algo_id,
                    0, // evaluate
                    fp.comments_ported as i32,
                    fp.basic_block_fixed_points.len() as i32,
                    0, // edges count matched
                    0, // instructions count matched
                ],
            ).context("Failed to insert function match")?;

            for bb in &fp.basic_block_fixed_points {
                let bb_algo_id = bb_algos.get(&bb.matching_step).cloned().unwrap_or(0);
                
                let addr1 = prim_fg.get_address(bb.primary_vertex);
                let addr2 = sec_fg.get_address(bb.secondary_vertex);

                tx.execute(
                    "INSERT INTO basicblock VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        basic_block_id,
                        function_id,
                        addr1 as i64,
                        addr2 as i64,
                        bb_algo_id,
                        0,
                    ],
                ).context("Failed to insert basic block match")?;

                basic_block_id += 1;
            }

            function_id += 1;
        }

        Ok(())
    }
}

fn get_clean_filename(path_str: &str) -> String {
    Path::new(path_str)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}
