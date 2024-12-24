import subprocess

RUCHES_RELEASE = "target/release/ruches"
REF_COMMAND = "stockfish"


positions = [
    "position startpos",
    "position fen 4r1k1/p1p2pp1/1q1p3p/1P3P2/1P6/2n1Q3/PB4PP/4R1K1 w - - 0 1",
    "position fen r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 0",
    "position fen r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "position fen rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "position fen r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
]

depths = [3, 4, 5, 6]


import subprocess
import time
import threading

s = []


def read_stdout(pipe):
    """Reads stdout incrementally and prints output as it arrives."""
    for line in iter(pipe.readline, ""):  # Read until EOF
        s.append(line)


def run_perft(command, pos, depth):
    global s
    s = []
    # Start the process
    process = subprocess.Popen(
        [command],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    # Start a thread to read stdout incrementally
    stdout_thread = threading.Thread(target=read_stdout, args=(process.stdout,))
    stdout_thread.daemon = True
    stdout_thread.start()
    # Send input after a delay (simulate late input)
    process.stdin.write("{}\n".format(pos))
    process.stdin.flush()

    time.sleep(0.25)  # leave time to load data before benchmarking

    t = time.time()

    process.stdin.write("go perft {}\n".format(depth))
    process.stdin.flush()
    process.stdin.write("quit\n")
    process.stdin.flush()

    # Wait for the process to complete
    process.wait()

    # Ensure all output is processed
    stdout_thread.join()
    dt = time.time() - t
    try:
        s = s[-2].split(" ")[-1][:-1]
    except:
        s = s[-1].split(" ")[-1][:-1]
    return s, dt


import numpy

x = [0] * 2

for i in range(len(positions)):
    p = positions[i]
    for d in depths:
        print("[Position index {}] \tTest depth d = {}... ".format(i + 1, d), end="")

        tst, dt_tst = run_perft(RUCHES_RELEASE, p, d)
        ref, dt_ref = run_perft(REF_COMMAND, p, d)

        tst_count = int(tst)
        ref_count = int(ref)
        if tst_count == ref_count:
            print("OK! ({})".format(tst_count))
            print("Time spent [new = {}, ref(stockfish) = {}]".format(dt_tst, dt_ref))
        else:
            print("Failed ({} instead of {}) [{}]".format(tst_count, ref_count, p))

        x[0] += dt_tst
        x[1] += dt_ref

x = [i / (len(positions) * len(depths)) for i in x]
print(
    "Average time spent in (seconds) : new = {}, ref(stockfish) = {}".format(x[0], x[1])
)
