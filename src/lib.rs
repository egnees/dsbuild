#![warn(missing_docs)]

//! DSbuild is a high-level framework aimed to provide foundation for building
//! distributed systems with support for simulation-based testing. The framework
//! guarantees system behaviour will be consistent in both virtual and real
//! modes, which can significantly simplify testing and debugging.
//!
//! ## Basic concepts
//!
//! DSbuild supports building aribitary distributed systems which are composed of
//! user-defined _processes_. Processes can interact with _network_, _file system_, _time_ and user.
//! Using framework, the implemented systems can run in the real word environment on
//! the single work station (_real mode_) or on multiple virtual _nodes_ within the _simulation_.
//!
//! **Real mode**. Using real mode, system can run in the real world environment with ability
//! to communicate in network, use file system and time. The primitives provided by DSBuild
//! guarantees system behaviour consistency in real mode and simulation.
//!
//! **Simulation**. The main purpose of simulation is to make process of testing distributed systems easy.
//! Network, time and file system are mocked in the simulation. It allows user to get
//! fine-grained control over them and set it's parameters according to the particular system specification.
//! User can create few virtual nodes and run the implemented distributed system on them. Framework guarantees
//! that behaviour of system in simulation will be the same as in real mode with respect to user settings of
//! virtual environment. This approach allows to test and debug system in very complicated scenarios
//! which can help user to find deep bugs and significantly increase system reliability. Also, as the sources of
//! system random behaviour are deterministically mocked in simulation, it guarantees reproducibility of results
//! from launch to launch.
//!
//! **Node**. Node hosts processes and manages network, file system and time. In simulation there can be several
//! nodes communicating in the virtual network, which are launched withing the single OS thread. User can disconnect it
//! from network or crash. In the real mode nodes can be launched on different physical hosts and communicate
//! in the real network. It both modes processes on the same node share access to network, file system and time.
//!
//! **Network**. In simulation, network represents abstraction other the nodes' communication environment.
//! User can control delay and drop rate of network. Also, simulation allows to make network partitions
//! or disconnect nodes from it.
//!
//! **File system**. File system represents abstraction over node's storage. In real mode it allows to
//! manipulate with files in the specified dirrectory. In simulation file system is virtual and allows
//! user to configure bandwith and other settings of the node storage.
//!
//! **Process**. Processes implemented by user define behaviour of the system. In particular, user can specify
//! response of process on previously set timer fired event or when process receives message from other
//! process via network. Process can interact with the outside world (simulation or real environment).
//! It particular, process can exchange messages with other processes using network,
//! request read or write to the node's file system, set timers and send local messages to user.
//! DSBuild allows processes to use both callbacks and asynchronous programming approach to cope with the
//! tasks listed above.

////////////////////////////////////////////////////////////////////////////////
mod real;

// Re-export public entities.
pub use real::io::IOProcessWrapper;
pub use real::node::Node as RealNode;

////////////////////////////////////////////////////////////////////////////////

mod sim;

// Re-export public entities.
pub use sim::system::Sim;

////////////////////////////////////////////////////////////////////////////////

mod common;

// Re-export public entities.
pub use common::{
    context::Context,
    fs::{File, FsError, FsResult},
    message::{Message, Tag},
    network::{SendError, SendResult},
    process::{Address, Process, ProcessGuard, ProcessWrapper},
};

pub use dsbuild_macros::Passable;
