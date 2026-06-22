use sysinfo::{Disks, Networks, ProcessesToUpdate, System};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Cpu,
    Mem,
    Name,
    Pid,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Cpu => "CPU",
            SortKey::Mem => "Mem",
            SortKey::Name => "Name",
            SortKey::Pid => "PID",
        }
    }
    pub fn next(self) -> Self {
        match self {
            SortKey::Cpu => SortKey::Mem,
            SortKey::Mem => SortKey::Name,
            SortKey::Name => SortKey::Pid,
            SortKey::Pid => SortKey::Cpu,
        }
    }
    pub fn to_index(self) -> usize {
        match self {
            SortKey::Cpu => 0,
            SortKey::Mem => 1,
            SortKey::Name => 2,
            SortKey::Pid => 3,
        }
    }
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => SortKey::Mem,
            2 => SortKey::Name,
            3 => SortKey::Pid,
            _ => SortKey::Cpu,
        }
    }
}

#[derive(Clone)]
pub struct ProcRow {
    pub pid: String,
    pub pid_num: u32,
    pub name: String,
    pub cpu: f32,
    pub mem: u64,
    pub run: u64,
    pub status: String,
}

pub struct SysMonitor {
    pub sys: System,
    pub networks: Networks,
    pub disks: Disks,
    pub net_rx: u64,
    pub net_tx: u64,
    pub disk_r: u64,
    pub disk_w: u64,
    pub cpu_history: Vec<u64>,
    pub rx_history: Vec<f64>,
    pub tx_history: Vec<f64>,
}

impl SysMonitor {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            net_rx: 0,
            net_tx: 0,
            disk_r: 0,
            disk_w: 0,
            cpu_history: vec![0; 100],
            rx_history: vec![0.0; 100],
            tx_history: vec![0.0; 100],
        }
    }

    pub fn tick(&mut self, tick: u64, refresh_ms: u64, sys_tab_active: bool) {
        if tick % 2 == 0 {
            self.sys.refresh_cpu_usage();
            self.sys.refresh_memory();
            let cpu = self.sys.global_cpu_usage().round().clamp(0.0, 100.0) as u64;
            self.cpu_history.remove(0);
            self.cpu_history.push(cpu);

            let secs = (2 * refresh_ms) as f64 / 1000.0;
            let per_sec = |bytes: u64| (bytes as f64 / secs) as u64;

            self.networks.refresh(true);
            let (mut rx, mut tx) = (0u64, 0u64);
            for data in self.networks.list().values() {
                rx += data.received();
                tx += data.transmitted();
            }
            self.net_rx = per_sec(rx);
            self.net_tx = per_sec(tx);

            // Keep history in MB/s
            self.rx_history.remove(0);
            self.rx_history.push(self.net_rx as f64 / 1_000_000.0);

            self.tx_history.remove(0);
            self.tx_history.push(self.net_tx as f64 / 1_000_000.0);

            if sys_tab_active {
                self.sys.refresh_processes(ProcessesToUpdate::All, true);

                self.disks.refresh(true);
                let (mut rd, mut wr) = (0u64, 0u64);
                for disk in self.disks.list() {
                    let u = disk.usage();
                    rd += u.read_bytes;
                    wr += u.written_bytes;
                }
                self.disk_r = per_sec(rd);
                self.disk_w = per_sec(wr);
            }
        }
    }
}

pub struct SocketRow {
    pub proto: String,
    pub local: String,
    pub remote: String,
    pub state: String,
}

pub fn get_tcp_sockets() -> Vec<SocketRow> {
    let mut rows = vec![];
    if let Ok(output) = std::process::Command::new("netstat")
        .arg("-an")
        .arg("-p")
        .arg("tcp")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                rows.push(SocketRow {
                    proto: parts[0].to_string(),
                    local: parts[3].to_string(),
                    remote: parts[4].to_string(),
                    state: parts[5].to_string(),
                });
            } else if parts.len() == 5 {
                rows.push(SocketRow {
                    proto: parts[0].to_string(),
                    local: parts[3].to_string(),
                    remote: parts[4].to_string(),
                    state: "".to_string(),
                });
            }
        }
    }
    rows
}
