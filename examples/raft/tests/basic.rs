use raft::{
    cmd::{CommandType, CREATED_CODE, UPDATED_CODE},
    local::LocalResponseType,
    sim::SimWrapper,
};

//////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn single_replica() {
    // create simulation with one node
    let mut sim = SimWrapper::new(12345, 1);

    // send init request
    sim.send_init_for_all();
    sim.make_steps_until_all_initialized();

    // make enough steps
    sim.make_steps(500);

    // check leader was elected
    assert_eq!(sim.current_leader(), Some(0));

    // check current term
    assert_eq!(sim.current_term(), Some(1));

    // send create and update command
    sim.send_command(0, CommandType::create("key1"));
    sim.make_steps(100);
    sim.send_command(0, CommandType::update("key1", "val1"));
    sim.make_steps(100);

    // assert create and update responses returned
    sim.process(0)
        .pop_local_and_expect_command_reply(CREATED_CODE);
    sim.process(0)
        .pop_local_and_expect_command_reply(UPDATED_CODE);
    sim.process(0).expect_no_local();

    // shutdown first node
    sim.shutdown_node(0);
    sim.make_steps(100);

    // rerun first node
    sim.rerun_node(0);
    sim.make_steps(100);
    assert_eq!(sim.current_term(), Some(2));
    assert_eq!(sim.current_leader(), Some(0));

    // wait for create and update replies
    sim.process(0)
        .pop_local_and_expect_command_reply(CREATED_CODE);
    sim.process(0)
        .pop_local_and_expect_command_reply(UPDATED_CODE);
    sim.process(0).expect_no_local();

    // send read requests and expect replies
    sim.send_read_request(0, "key1");
    sim.send_read_request(0, "key2");
    sim.make_steps(100);
    sim.process(0).pop_local_and_expect_read_value(Some("val1"));
    sim.process(0).pop_local_and_expect_read_value(None);
    sim.process(0).expect_no_local();
}

//////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn seq_numbers_correct() {
    // create sim and make enough steps until all initialized
    let mut sim = SimWrapper::new(12345, 1);
    sim.send_init_for_all();
    sim.make_steps_until_all_initialized();

    // elect leader
    sim.make_steps(100);
    assert_eq!(sim.current_leader(), Some(0));
    assert_eq!(sim.current_term(), Some(1));

    // send commands
    let c1 = sim.send_command(0, CommandType::create("k1"));
    let c2 = sim.send_command(0, CommandType::update("k1", "v1"));
    assert_eq!(c2.sequence_number(), c1.sequence_number() + 1);
    let c3 = sim.send_read_request(0, "k1");
    assert_eq!(c3.sequence_number(), c2.sequence_number() + 1);

    // make steps
    sim.make_steps(500);

    // read responses
    let proc = sim.process(0);
    proc.pop_local_and_expect_command_id(c1);
    proc.pop_local_and_expect_command_id(c2);
    proc.pop_local_and_expect_command_id(c3);

    // rerun node
    sim.shutdown_node(0);
    sim.make_steps(100);
    sim.rerun_node(0);
    sim.make_steps(100);
    assert_eq!(sim.current_leader(), Some(0));
    assert_eq!(sim.current_term(), Some(2));

    // expect previous commands
    assert_eq!(sim.process(0).locals_len(), 2);
    sim.process(0).clear_locals();

    let c4 = sim.send_command(0, CommandType::create("k2"));
    assert_eq!(c4.sequence_number(), c3.sequence_number() + 1);

    // make steps
    sim.make_steps(100);

    // read response
    sim.process(0).pop_local_and_expect_command_id(c4);
}

//////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn three_replicas() {
    // there are three replicas
    // after election, i send one update on leader
    // then i check:
    //  1. followers redirects requests to leader
    //  2. leader redirects read requests to followers with up-to-date commit index

    // create sim and make steps until all initialzed
    let mut sim = SimWrapper::new(12345, 3);
    sim.send_init_for_all();
    sim.make_steps_until_all_initialized();

    // elect leader
    sim.make_steps(200);
    assert_eq!(sim.current_term(), Some(1));
    let leader = sim.current_leader().expect("leader must be elected");

    // send create command
    sim.send_command(leader, CommandType::create("k1"));
    sim.make_steps_until_local_message(leader);
    sim.process(leader)
        .pop_local_and_expect_command_reply(CREATED_CODE);

    // forward simulation
    sim.make_steps(200);

    // send requests to followers and expect redirection
    for not_leader in (1..=2).map(|i| (i + leader) % 3) {
        // send create command
        sim.send_command(not_leader, CommandType::create("k2"));
        sim.make_steps_until_local_message(not_leader);
        sim.process(not_leader)
            .pop_local_and_expect_redirected_to(leader, None);

        // send read command
        sim.send_read_request(not_leader, "k1");
        sim.make_steps_until_local_message(not_leader);
        sim.process(not_leader)
            .pop_local_and_expect_redirected_to(leader, None);
    }

    // forward simulation
    sim.make_steps(200);

    // try read values 9 times from leader and
    // expect 3 answers and 6 redirections to other replicas
    let mut replies = (0..9)
        .map(|_| {
            sim.send_read_request(leader, "k1");
            sim.make_steps_until_local_message(leader);
            let response = sim.process(leader).pop_next_local().unwrap();
            match response.tp {
                LocalResponseType::ReadValue(value) => (leader, value),
                LocalResponseType::RedirectedTo(to, Some(commit_idx)) => {
                    sim.send_read_request_with_commit_idx(to, "k1", commit_idx);
                    sim.make_steps_until_local_message(to);
                    let response = sim.process(to).pop_next_local().unwrap();
                    if let LocalResponseType::ReadValue(value) = response.tp {
                        (to, value)
                    } else {
                        panic!("unexpected local response type")
                    }
                }
                _ => panic!("unexpected local response type"),
            }
        })
        .collect::<Vec<_>>();
    replies.sort();

    // check all nine requests were answered by different replicas (3 by 3)
    assert_eq!(
        replies,
        (0..9)
            .map(|i| (i / 3, Some(String::new())))
            .collect::<Vec<_>>()
    );
}

//////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn reelection() {
    // shutdown leader five times and check new leader and term appears

    let mut sim = SimWrapper::new(321, 3);
    sim.send_init_for_all();
    sim.make_steps_until_all_initialized();

    // forward simulation
    sim.make_steps(200);

    // check election
    let mut current_term = sim.current_term().unwrap();
    assert_eq!(current_term, 1);
    let mut current_leader = sim.current_leader().unwrap();

    // check leader reelection five times
    for _ in 0..5 {
        sim.shutdown_node(current_leader);
        sim.make_steps(200);
        let next_term = sim.current_term().unwrap();
        assert!(next_term > current_term);
        let next_leader = sim.current_leader().unwrap();
        assert_ne!(current_leader, next_leader);

        sim.rerun_node(current_leader);
        current_term = next_term;
        current_leader = next_leader;
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn log_replication() {
    // check log replication is consistent
    // send commands to current leader and check
    // they and persistent after leader fails
    let mut sim = SimWrapper::new(333, 3);
    sim.send_init_for_all();
    sim.make_steps_until_all_initialized();

    // forward sim
    sim.make_steps(200);

    // get current leader and current term
    let mut current_leader = sim.current_leader().unwrap();
    let mut current_term = sim.current_term().unwrap();

    // kill current leader 10 times and
    // check commited commands are persistent
    for iter in 1..=10 {
        // shutdown current leader
        sim.shutdown_node(current_leader);

        // make election
        sim.make_steps(500);

        // check election is correct
        let next_leader = sim.current_leader().unwrap();
        let next_term = sim.current_term().unwrap();
        assert!(next_term > current_term);
        assert_ne!(current_leader, next_leader);

        // update state
        // store prev leader to be able to reboot it
        let prev_leader = current_leader;
        current_leader = next_leader;
        current_term = next_term;

        // apply new command and check previous
        // commands applied too

        // create new value
        let create_id = sim.send_command(
            current_leader,
            CommandType::create(format!("k{}", iter).as_str()),
        );
        sim.make_steps_until_response_id(current_leader, create_id);
        sim.process(current_leader)
            .pop_local_and_expect_command_reply(CREATED_CODE);

        // update new value
        let update_id = sim.send_command(
            current_leader,
            CommandType::update(format!("k{}", iter).as_str(), format!("v{}", iter).as_str()),
        );
        sim.make_steps_until_response_id(current_leader, update_id);
        sim.process(current_leader)
            .pop_local_and_expect_command_reply(UPDATED_CODE);

        // send read requests and wait responses
        for (key, value) in (1..=iter).map(|n| (format!("k{}", n), format!("v{}", n))) {
            let read_id = sim.send_read_request(current_leader, &key);
            sim.make_steps_until_response_id(current_leader, read_id);
            let response = sim.process(current_leader).pop_next_local().unwrap();
            let read_value = match response.tp {
                LocalResponseType::ReadValue(value) => value,
                LocalResponseType::RedirectedTo(to, Some(commit_index)) => {
                    sim.send_read_request_with_commit_idx(to, &key, commit_index);
                    sim.make_steps_until_local_message(to);
                    sim.process(to).pop_local_and_assure_read_value()
                }
                _ => panic!("unexpected response from leader"),
            };
            assert_eq!(read_value, Some(value));
        }

        // reboot shutdown node
        sim.rerun_node(prev_leader);

        // forward sim
        sim.make_steps(500);
    }
}

//////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn network_split() {
    // split network and check nothing does wrong

    let mut sim = SimWrapper::new(12345, 3);
    sim.send_init_for_all();
    sim.make_steps_until_all_initialized();

    // election
    sim.make_steps(400);

    // send updates to leader
    let leader = sim.current_leader().unwrap();
    let term = sim.current_term().unwrap();

    // create
    let create = sim.send_command(leader, CommandType::create("k1"));
    sim.make_steps_until_response_id(leader, create);
    sim.process(leader)
        .pop_local_and_expect_command_reply(CREATED_CODE);

    // update
    let update = sim.send_command(leader, CommandType::update("k1", "v1"));
    sim.make_steps_until_response_id(leader, update);
    sim.process(leader)
        .pop_local_and_expect_command_reply(UPDATED_CODE);

    // disconnect leader from network
    sim.split_network(&[leader]);

    // make one more update on leader and expect no updates will be committed.
    sim.send_command(leader, CommandType::update("k1", "v2"));
    sim.make_steps(500);
    sim.process(leader).expect_no_local();

    // meanwhile new leader must be elected
    let new_leader = sim.current_leader().unwrap();
    let new_term = sim.current_term().unwrap();
    assert!(new_term > term);
    assert!(new_leader != leader);

    // read value from new leader
    sim.send_read_request(new_leader, "k1");
    sim.make_steps_until_local_message(new_leader);
    let response = sim.process(new_leader).pop_next_local().unwrap();
    let read_value = match response.tp {
        LocalResponseType::ReadValue(value) => value,
        LocalResponseType::RedirectedTo(to, Some(commit_index)) => {
            sim.send_read_request_with_commit_idx(to, "k1", commit_index);
            sim.make_steps_until_local_message(to);
            sim.process(to).pop_local_and_assure_read_value()
        }
        _ => panic!("unexpected response type"),
    };
    assert_eq!(read_value, Some("v1".to_owned()));

    // update k1 with v3
    let update = sim.send_command(new_leader, CommandType::update("k1", "v3"));
    sim.make_steps_until_response_id(new_leader, update);
    sim.process(new_leader)
        .pop_local_and_expect_command_reply(UPDATED_CODE);

    // repair network
    sim.repair_network();

    // forward simulation
    sim.make_steps(500);

    // read value again and assure it equals v3 and not v2
    sim.send_read_request(new_leader, "k1");
    sim.make_steps_until_local_message(new_leader);
    let response = sim.process(new_leader).pop_next_local().unwrap();
    let read_value = match response.tp {
        LocalResponseType::ReadValue(value) => value,
        LocalResponseType::RedirectedTo(to, Some(commit_index)) => {
            sim.send_read_request_with_commit_idx(to, "k1", commit_index);
            sim.make_steps_until_local_message(to);
            sim.process(to).pop_local_and_assure_read_value()
        }
        _ => panic!("unexpected response type"),
    };
    assert_eq!(read_value, Some("v3".to_owned()));
}
