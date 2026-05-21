# Erlang (programming language)

**Erlang** is a general-purpose, concurrent, functional high-level programming language, and a garbage-collected runtime system. The term Erlang is used interchangeably with Erlang/OTP, or Open Telecom Platform (OTP), which consists of the Erlang runtime system, several ready-to-use components (OTP) mainly written in Erlang, and a set of design principles for Erlang programs.

The Erlang runtime system is designed for systems with these traits:

- Distributed
- Fault-tolerant
- Soft real-time
- Highly available, non-stop applications
- Hot swapping, where code can be changed without stopping a system.

The Erlang programming language has data, pattern matching, and functional programming.

## History

The name Erlang, attributed to Bjarne Däcker, has been presumed by those working at the Ericsson Computer Science Laboratory to be a reference to Danish mathematician and engineer Agner Krarup Erlang and a syllabic abbreviation of "Ericsson language". Erlang was designed with the aim of improving the development of telephony applications.

The initial version of Erlang was implemented in Prolog and was influenced by the programming language PLEX used in earlier Ericsson exchanges. By 1988 Erlang had proven suitable for prototyping telephone exchanges, but the Prolog interpreter was far too slow.

A faster implementation of Erlang, BEAM, was developed starting in 1992. BEAM compiles Erlang programs into C using a register-based abstract machine. BEAM was incorporated into the Erlang runtime system in 1995. As a result, Erlang was chosen for the next Asynchronous Transfer Mode (ATM) exchange AXD.

In February 1998, Ericsson Radio Systems banned the in-house use of Erlang for new products, citing a preference for non-proprietary languages. The ban caused Armstrong and others to make plans to leave Ericsson. In March 1998 Ericsson announced the AXD301 switch, containing over a million lines of Erlang and reported to achieve a high availability of nine "9"s. In December 1998, the implementation of Erlang was open-sourced and most of the Erlang team resigned to form a new company, Bluetail AB. Ericsson eventually relaxed the ban and re-hired Armstrong in 2004.

In 2006, native symmetric multiprocessing support was added to the runtime system and VM.

## Processes

Erlang applications are built of very lightweight Erlang processes in the Erlang runtime system. The Erlang runtime system provides strict process isolation between Erlang processes (this includes data and garbage collection, separated individually by each Erlang process) and transparent communication between processes on different Erlang nodes (on different hosts).

Joe Armstrong, co-inventor of Erlang, summarized the principles of processes in his PhD thesis:

- Everything is a process.
- Processes are strongly isolated.
- Process creation and destruction is a lightweight operation.
- Message passing is the only way for processes to interact.
- Processes have unique names.
- If you know the name of a process you can send it a message.
- Processes share no resources.
- Error handling is non-local.
- Processes do what they are supposed to do or fail.

## Usage

In 2014, Ericsson reported Erlang was being used in its support nodes, and in GPRS, 3G and LTE mobile networks worldwide and also by Nortel and Deutsche Telekom.

## Supervisor trees

A typical Erlang application is written in the form of a supervisor tree. This architecture is based on a hierarchy of processes in which the top level process is known as a "supervisor". The supervisor then spawns multiple child processes that act either as workers or more, lower level supervisors.

Within a supervisor tree, all supervisor processes are responsible for managing the lifecycle of their child processes, and this includes handling situations in which those child processes crash.

## Concurrency and distribution orientation

Erlang's main strength is support for concurrency. It has a small but powerful set of primitives to create processes and communicate among them. Erlang is conceptually similar to the language occam, though it recasts the ideas of communicating sequential processes (CSP) in a functional framework and uses asynchronous message passing.

Processes are the primary means to structure an Erlang application. They are neither operating system processes nor threads, but lightweight processes that are scheduled by BEAM. Like operating system processes (but unlike operating system threads), they share no state with each other.

While threads need external library support in most languages, Erlang provides language-level features to create and manage processes with the goal of simplifying concurrent programming. Though all concurrency is explicit in Erlang, processes communicate using message passing instead of shared variables, which removes the need for explicit locks.

Although Erlang was designed to fill a niche and has remained an obscure language for most of its existence, its popularity is growing due to demand for concurrent services.
