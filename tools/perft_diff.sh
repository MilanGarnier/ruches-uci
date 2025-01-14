#!/bin/sh

DEPTH="$1"
POSITION="$2"

SF_OUT="sf.perft_out"
RS_OUT="rs.perft_out"
TEMP="temp.txt"

# Send commands to stockfish
(echo "$POSITION"; echo "go perft $DEPTH"; echo "quit") | stockfish > $SF_OUT

# Send commands to ruches
(echo "$POSITION"; echo "go perft $DEPTH"; echo "quit") | $(pwd)/target/release/ruches > $RS_OUT

# Process stockfish output
awk '/^[a-h][1-8][a-h][1-8]/ {gsub(/:|,/,""); print $1, $2}' $SF_OUT | sort > $SF_OUT.processed
echo "Processed stockfish output:"

# Process ruches output
awk '/^[a-h][1-8][a-h][1-8]/ {gsub(/:|,/,""); print $1, $2}' $RS_OUT | sort > $RS_OUT.processed
echo "Processed ruches output:"

echo "SF <- -> Ruches"
sdiff $SF_OUT.processed $RS_OUT.processed

# Cleanup
rm -f $SF_OUT.processed $RS_OUT.processed
