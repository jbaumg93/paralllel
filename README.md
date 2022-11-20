# Paralllel

A: _Have you ever tried [GNU parallel](https://www.gnu.org/software/parallel/)? Did it work for you?_  
B: _Ehm, yes._  
A: _That's great, how many hours did you spent? Can you add more threads during runtime? Did it show you how long it takes to finish?_  
B: _... You are weird._  

_Paralllel_ is a minimalistic and easy to use parallel job executor. It supports adding more threads and reducing threads during runtime. It even calculates an ETA. It's written in [Rust](https://www.rust-lang.org) (_Yey!_) and not in [Perl](https://www.youtube.com/watch?v=noQcWra6sbU) (_Buhh!_)!

## Usage

    Usage: paralllel [OPTIONS] <COMMAND>

    Arguments:
        <COMMAND>  Execute this command (in parallel), replaces {N} with the Nth value (index starts at 0)

    Options:
        -i, --input-file <INPUT_FILE>        Name of the input file (if omitted, stdin is used)
        -t, --threads <THREADS>              Number of threads initially created [default: 1]
        -d, --delimiter <DELIMITER>          Delimiter for input values [default: "\t"]
        -s, --show-output-instead-of-status  Does not show the progress, prints stdout and stderr of the subprocesses
        -h, --help                           Print help information
        -V, --version                        Print version information

## Minimal example

Launch three parameterized sleeps in parallel (with two threads):

    $ printf '1\n2\n3' | paralllel -t 2 'sleep {0}'

_It's that simple!_

## A more advanced example

For the following example we use this as the input file (corresponds to four jobs):

    $ cat test-input.txt
    1|/
    2|/tmp
    2|.
    3|/

Per default, each value is separated by tabs but here we use `-d` to set a custom delimiter. To start with three threads and no status bar (stdout and stderr as output) run the following:

    $ paralllel -i test-input.txt -d '|' -t 3 -s 'sleep {0} && ls {1}'

Again, _It's that simple!_

BTW, Ctrl+C'ing _paralllel_ does not SIGINT the subprocesses, _paralllel_ waits for the current jobs to finish and then exits. In other words: Ctrl+C means no new jobs. 

To save the output of each job while still printing it, just use [`tee`](https://man7.org/linux/man-pages/man1/tee.1.html):

    $ paralllel -i test-input.txt -d '|' -t 2 -s 'sleep {0} && ls {1} | tee /tmp/{0}_$(date +%s).txt'

## Increase and decrease threads

Use SIGUSR1 to increase and SIGUSR2 to decrease the number of threads by one:

    $ printf '2\n2\n2\n2\n2\n2\n2\n2\n2\n2\n' | paralllel 'sleep {0}'
    $
    $ # In another terminal:
    $ kill -SIGUSR1 $(pgrep -f paralllel)  # to increase threads or
    $ kill -SIGUSR2 $(pgrep -f paralllel)  # to to decrease threads

Currently running threads are not affected when decreasing the number of threads.

## Clone, build and install

We use [rustup](https://rustup.rs). Clone, build and install:

    git clone https://github.com/jbaumg93/paralllel
    cd paralllel
    cargo b --release
    cp target/release/paralllel [SOME_$PATH_YOU_LIKE]

## Known bugs

- Status bar can be incorrect near the end due to the final join(). 