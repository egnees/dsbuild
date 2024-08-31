#![warn(missing_docs)]

//! DSbuild is a high-level framework aimed to provide foundation for building
//! distributed systems with support for simulation based testing. The framework
//! guarantees system behaviour will be consistent in both simulation and real
//! modes, which can significantly simplify testing and debugging.
//!
//! ## Basic concepts
//!
//! DSbuild supports building aribitary distributed systems which are composed of
//! user-defined _processes_. These processes can interact with _network_, _file system_, _time_ and user.
//! Using framework, the implemented systems can be run in the _real word_ environment on
//! the single work station or on multiple virtual _nodes_ inside of _simulation_.
//!
//! **Real mode**. Using real mode, system can be run in the real world environment with ability
//! to communicate in network, use file system and time. The primitives provided by DSBuild
//! guarantees their consistency in the real mode and simulation.
//!
//! **Simulation**. The main purpose of the simulation (a.k.a virtual mode) is to make easy process of testing and
//! debugging of distributed systems. Network, time and filesystem are mocked in the simulation. It allows user to get
//! fine-grained control over them and set it's parameters according to the particular system specification. User can
//! create few virtual nodes and run the implemented distributed system on them. The framework guarantees that behaviour
//! of the system in the simulation will be approximately the same as in the real mode with respect to user settings of
//! virtual environment. This approach allows to test and debug behaviour of the system in very complicated scenarios,
//! which helps user to find the deepest bugs and significantly increase system's reliability. Also, as the sources of
//! system's random behaviour are deterministically mocked in the simulation, it gurantees reproducibility of results
//! from launch to launch.
//!
//! **Node**. Node represents the unit of system. In simulation can be several nodes, communicating in the network.
//! In the real mode there is the only one node. Processes on the same node share access to the network, file system
//! and time. In simulation, user can disconnect node from network, crash it or crash it's storage.
//! As a result, node can become unreachable for other nodes.
//!
//! **Network**. In simulation, network represents abstraction other the nodes communication environment.
//! User can control delays and drop rate of the network. Also, simulation allows to split network
//! or disconnect nodes from it.
//!
//! **File system**. File system represents abstraction over node's storage. In real mode it allows to
//! manipulate with files in the specified dirrectory. In the virtual mode, file system simulated
//! in the RAM. Simulation allows user to configure bandwith and other settings of the node storage.
//!
//! **Process**. Processes implemented by user defines behaviour of the system. In particular, user can specify
//! reaction of the process on previously set timer fire or when process receives message from other
//! process via network. Process can interact with the outside world (simulation or real envinroment).
//! It particular, process can exchange messages with other processes using network,
//! request reading or writing to the node's file system, set timers and send local messages to user.
//! DSBuild allows processes to use both callbacks and asynchronous programming approach to take deal with the
//! tasks listed above.

////////////////////////////////////////////////////////////////////////////////
mod real;

// Re-export public entities.
pub use real::io::IOProcessWrapper;
pub use real::node::Node as RealNode;

////////////////////////////////////////////////////////////////////////////////

mod sim;

// Re-export public entities.
pub use sim::system::Sim as VirtualSystem;

////////////////////////////////////////////////////////////////////////////////

mod common;
pub use common::storage;

// Re-export public entities.
pub use common::{
    context::Context,
    message::Message,
    network::{SendError, SendResult},
    process::{Address, Process, ProcessGuard, ProcessWrapper},
    tag::Tag,
};
