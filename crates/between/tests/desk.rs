//! An owner's desk: commands and questions go to the one task that owns the state, are carried
//! out there one at a time, and every asker is answered or refused by a code within its limit.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use between::{Desk, Refused};

/// An owner that serves its desk for FOR, taking TAKES per request, and writes down which thread answered each.
fn owner<const LEN: usize, const PLACES: usize>(
    desk: &'static Desk<u32, u32, LEN, PLACES>,
    serves_for: Duration,
    takes: Duration,
) -> thread::JoinHandle<Vec<thread::ThreadId>> {
    thread::spawn(move || {
        let busy = AtomicBool::new(false);
        let mut answered_in = Vec::new();
        let until = Instant::now() + serves_for;
        while Instant::now() < until {
            desk.serve_for(until.saturating_duration_since(Instant::now()), |asked| {
                assert!(!busy.swap(true, Ordering::SeqCst), "two requests were carried out at once");
                thread::sleep(takes);
                busy.store(false, Ordering::SeqCst);
                answered_in.push(thread::current().id());
                asked * 10
            });
        }
        answered_in
    })
}

#[test]
fn two_asked_at_once_are_carried_out_one_after_the_other_and_both_are_answered() {
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let serving = owner(&DESK, Duration::from_millis(600), Duration::from_millis(50));
    let start = Arc::new(Barrier::new(2));
    let askers: Vec<_> = [1u32, 2]
        .into_iter()
        .map(|what| {
            let start = start.clone();
            thread::spawn(move || {
                start.wait();
                DESK.ask(what, Duration::from_millis(400))
            })
        })
        .collect();
    let answers: Vec<_> = askers.into_iter().map(|a| a.join().unwrap()).collect();
    assert_eq!(answers, [Ok(10), Ok(20)], "each asker gets the answer to its own question");
    let answered_in = serving.join().unwrap();
    assert_eq!(answered_in.len(), 2, "both were carried out");
}

#[test]
fn an_owner_that_does_not_answer_is_refused_as_silent_when_the_limit_runs_out() {
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let began = Instant::now();
    let answer = DESK.ask(1, Duration::from_millis(100));
    let took = began.elapsed();
    assert_eq!(answer, Err(Refused::Silent));
    assert!(took >= Duration::from_millis(100), "refused before its limit: {took:?}");
    assert!(took < Duration::from_millis(600), "hung past its limit: {took:?}");
}

#[test]
fn an_owner_that_answers_in_time_is_not_refused() {
    // the pair of the silence: the same limit, an owner that is there
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    let serving = owner(&DESK, Duration::from_millis(300), Duration::from_millis(5));
    assert_eq!(DESK.ask(3, Duration::from_millis(100)), Ok(30));
    serving.join().unwrap();
}

#[test]
fn a_full_queue_refuses_at_once_as_busy() {
    static DESK: Desk<u32, u32, 1, 2> = Desk::new();
    // nobody serves: the first asking fills the one place in the queue and waits there
    let first = thread::spawn(|| DESK.ask(1, Duration::from_millis(500)));
    thread::sleep(Duration::from_millis(50));
    let began = Instant::now();
    assert_eq!(DESK.ask(2, Duration::from_millis(500)), Err(Refused::Busy));
    assert!(began.elapsed() < Duration::from_millis(100), "busy is said at once, not after the limit");
    assert_eq!(first.join().unwrap(), Err(Refused::Silent));
}

#[test]
fn a_late_answer_to_one_asker_does_not_reach_the_next() {
    static DESK: Desk<u32, u32, 4, 1> = Desk::new();
    // the owner takes 200 ms over each: the first asker gives up after 50, and the second takes its place
    let serving = owner(&DESK, Duration::from_millis(900), Duration::from_millis(200));
    assert_eq!(DESK.ask(1, Duration::from_millis(50)), Err(Refused::Silent));
    assert_eq!(DESK.ask(2, Duration::from_millis(700)), Ok(20), "the second asker got the first one's answer");
    serving.join().unwrap();
}

#[test]
fn more_askers_than_places_wait_for_a_place_and_are_all_answered() {
    static DESK: Desk<u32, u32, 4, 1> = Desk::new();
    let serving = owner(&DESK, Duration::from_millis(600), Duration::from_millis(30));
    let start = Arc::new(Barrier::new(3));
    let askers: Vec<_> = [1u32, 2, 3]
        .into_iter()
        .map(|what| {
            let start = start.clone();
            thread::spawn(move || {
                start.wait();
                DESK.ask(what, Duration::from_millis(400))
            })
        })
        .collect();
    let answers: Vec<_> = askers.into_iter().map(|a| a.join().unwrap()).collect();
    assert_eq!(answers, [Ok(10), Ok(20), Ok(30)]);
    serving.join().unwrap();
}

/// The bus and its round, as the burner's temperatures keep them: the owner polls with the bus in hand, and between
/// rounds answers its desk - a scan asked in the middle of a poll waits for the poll and runs in the owner's task.
#[test]
fn a_request_during_the_owners_own_work_waits_for_it_and_is_carried_out_in_the_owners_task() {
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    static POLLS_DONE: AtomicU32 = AtomicU32::new(0);
    let poll_ended: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let ended = poll_ended.clone();
    let owner = thread::spawn(move || {
        let me = thread::current().id();
        let mut scans = Vec::new();
        for _ in 0..3 {
            // the poll, with the bus in this task's hand
            thread::sleep(Duration::from_millis(100));
            ended.lock().unwrap().get_or_insert(Instant::now());
            POLLS_DONE.fetch_add(1, Ordering::SeqCst);
            DESK.serve_for(Duration::from_millis(20), |what| {
                scans.push((thread::current().id(), Instant::now()));
                what + 1
            });
        }
        (me, scans)
    });
    // ask in the middle of the first poll
    thread::sleep(Duration::from_millis(40));
    assert_eq!(DESK.ask(7, Duration::from_millis(500)), Ok(8));
    assert!(POLLS_DONE.load(Ordering::SeqCst) >= 1, "answered before the poll it came in during had ended");
    let (me, scans) = owner.join().unwrap();
    assert_eq!(scans.len(), 1);
    assert_eq!(scans[0].0, me, "the scan ran in another task than the owner's");
    let first_poll_ended = poll_ended.lock().unwrap().expect("the first poll ended");
    assert!(scans[0].1 >= first_poll_ended, "the scan ran before the poll it came in during had ended");
}

#[test]
fn a_request_whose_asker_gave_up_is_not_carried_out() {
    static DESK: Desk<u32, u32, 4, 2> = Desk::new();
    static CARRIED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    // asked while nobody serves: refused as silent, and the request is still in the queue
    assert_eq!(DESK.ask(1, Duration::from_millis(50)), Err(Refused::Silent));
    // the owner comes up late, and is asked again
    let serving = thread::spawn(|| {
        DESK.serve_for(Duration::from_millis(300), |what| {
            CARRIED.lock().unwrap().push(what);
            what * 10
        })
    });
    assert_eq!(DESK.ask(2, Duration::from_millis(200)), Ok(20));
    serving.join().unwrap();
    assert_eq!(*CARRIED.lock().unwrap(), [2], "a command refused to its asker was carried out after all");
}
