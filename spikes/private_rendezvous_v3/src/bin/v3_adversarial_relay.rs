//! Independent-process adversarial relay schedule for the isolated v3 model.
//!
//! This executable intentionally demonstrates both fail-closed schedules and
//! the indistinguishability counterexample where a malicious relay hides a
//! third participant. It is test evidence, not a network service.

use std::{collections::BTreeMap, env, process::ExitCode};

use scotoma_private_rendezvous_v3_model::{
    Phase, PublicConfig, ResearchSession, ThresholdResponse,
};

const MANIFEST: [u8; 32] = [0x53; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Honest,
    HideCollision,
    DelayCollision,
    DuplicateCollision,
    ReorderBeforePair,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "honest" => Ok(Self::Honest),
            "hide" => Ok(Self::HideCollision),
            "delay" => Ok(Self::DelayCollision),
            "duplicate" => Ok(Self::DuplicateCollision),
            "reorder" => Ok(Self::ReorderBeforePair),
            _ => Err(format!("unknown mode: {value}")),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Honest => "honest",
            Self::HideCollision => "hide",
            Self::DelayCollision => "delay",
            Self::DuplicateCollision => "duplicate",
            Self::ReorderBeforePair => "reorder",
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            println!(
                "{{\"schema\":\"scotoma-v3-adversarial-relay-v1\",\"mode\":\"{}\",\"order\":\"{}\",\"confirmed\":{},\"terminal\":{},\"collision_deliveries\":{},\"ambiguous_pair_confirmed\":{}}}",
                report.mode.name(),
                report.order,
                report.confirmed,
                report.terminal,
                report.collision_deliveries,
                report.ambiguous_pair_confirmed
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

struct Report {
    mode: Mode,
    order: String,
    confirmed: usize,
    terminal: usize,
    collision_deliveries: usize,
    ambiguous_pair_confirmed: bool,
}

fn run() -> Result<Report, String> {
    let (mode, order) = parse_args()?;
    let mut clients = order
        .iter()
        .map(|participant| Ok((*participant, waiting_session()?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let first = order[0];
    let second = order[1];
    let third = order[2];
    let mut collision_deliveries = 0;

    match mode {
        Mode::Honest => {
            pair(&mut clients, first, second)?;
            deliver_collision(&mut clients, first);
            deliver_collision(&mut clients, second);
            deliver_collision(&mut clients, third);
            collision_deliveries = 3;
        }
        Mode::HideCollision => {
            pair(&mut clients, first, second)?;
            confirm(&mut clients, first)?;
            confirm(&mut clients, second)?;
            terminate_third(&mut clients, third);
        }
        Mode::DelayCollision => {
            pair(&mut clients, first, second)?;
            confirm(&mut clients, first)?;
            confirm(&mut clients, second)?;
            deliver_collision(&mut clients, first);
            deliver_collision(&mut clients, second);
            deliver_collision(&mut clients, third);
            collision_deliveries = 3;
        }
        Mode::DuplicateCollision => {
            pair(&mut clients, first, second)?;
            for participant in [first, second, third] {
                deliver_collision(&mut clients, participant);
                deliver_collision(&mut clients, participant);
                collision_deliveries += 2;
            }
        }
        Mode::ReorderBeforePair => {
            // A confirmation/frame delivered before the Paired event is a
            // protocol-order violation and makes each affected client terminal.
            let _ = clients
                .get_mut(&first)
                .ok_or("missing first client")?
                .confirm(true);
            let _ = clients
                .get_mut(&second)
                .ok_or("missing second client")?
                .confirm(true);
            deliver_collision(&mut clients, third);
            collision_deliveries = 1;
        }
    }

    let confirmed = clients
        .values()
        .filter(|client| client.phase() == Phase::Confirmed)
        .count();
    let terminal = clients
        .values()
        .filter(|client| client.phase() == Phase::Terminal)
        .count();
    Ok(Report {
        mode,
        order: order
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("-"),
        confirmed,
        terminal,
        collision_deliveries,
        ambiguous_pair_confirmed: confirmed == 2,
    })
}

fn waiting_session() -> Result<ResearchSession, String> {
    let mut session = ResearchSession::new(PublicConfig {
        manifest_hash: MANIFEST,
        epoch: 7,
        operator_count: 3,
        threshold: 2,
    })
    .map_err(|error| format!("session setup failed: {error:?}"))?;
    for operator_id in [0, 1] {
        session
            .accept_threshold_response(ThresholdResponse {
                operator_id,
                manifest_hash: MANIFEST,
                epoch: 7,
                proof_valid: true,
            })
            .map_err(|error| format!("threshold setup failed: {error:?}"))?;
    }
    session
        .register_with_relay()
        .map_err(|error| format!("relay registration failed: {error:?}"))?;
    Ok(session)
}

fn pair(
    clients: &mut BTreeMap<u64, ResearchSession>,
    first: u64,
    second: u64,
) -> Result<(), String> {
    clients
        .get_mut(&first)
        .ok_or("missing first client")?
        .pair([0x11; 32], [0x22; 32], 2)
        .map_err(|error| format!("first pair failed: {error:?}"))?;
    clients
        .get_mut(&second)
        .ok_or("missing second client")?
        .pair([0x22; 32], [0x11; 32], 2)
        .map_err(|error| format!("second pair failed: {error:?}"))?;
    Ok(())
}

fn confirm(clients: &mut BTreeMap<u64, ResearchSession>, participant: u64) -> Result<(), String> {
    clients
        .get_mut(&participant)
        .ok_or("missing client")?
        .confirm(true)
        .map_err(|error| format!("confirmation failed: {error:?}"))
}

fn deliver_collision(clients: &mut BTreeMap<u64, ResearchSession>, participant: u64) {
    let _ = clients
        .get_mut(&participant)
        .expect("scenario participant exists")
        .collision();
}

fn terminate_third(clients: &mut BTreeMap<u64, ResearchSession>, participant: u64) {
    let _ = clients
        .get_mut(&participant)
        .expect("scenario participant exists")
        .timeout();
}

fn parse_args() -> Result<(Mode, [u64; 3]), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [mode_flag, mode, order_flag, order] = args.as_slice() else {
        return Err("usage: --mode <honest|hide|delay|duplicate|reorder> --order <a,b,c>".into());
    };
    if mode_flag != "--mode" || order_flag != "--order" {
        return Err("invalid arguments".into());
    }
    let mode = Mode::parse(mode)?;
    let participants = order
        .split(',')
        .map(|value| value.parse::<u64>().map_err(|_| "invalid participant"))
        .collect::<Result<Vec<_>, _>>()?;
    let order: [u64; 3] = participants
        .try_into()
        .map_err(|_| "order must contain exactly three participants")?;
    if order[0] == order[1] || order[0] == order[2] || order[1] == order[2] {
        return Err("participants must be distinct".into());
    }
    Ok((mode, order))
}
