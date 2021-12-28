# CFR Implementation Scrabble (Work in Progress)

Implementation of vanilla CFR and monte-carlo (outcome sampling) for the game of scrabble.

I empirically evaluated the algorithm on tic-tac-toe because it was very simple to code up and trains
in seconds. The scrabble implementation is still a work in progress and very much in the design phase, but
if I were to apply it to CFR I would follow this approach:

State Space: Board can be encoded in a 15x15 grid where each cell is a single character.
This means that we can still store keys as strings and index them into a hash table. This makes state space lookups very efficient.

Action Space:

This is the hardest part and I still don't have a good design. Most scrabble solvers will greedily select a move that maximizes their score. Playing against the vast majority of players (myself and my family included), this will probably work. However, there are other strategies that involve cutting other players off from both forming words and stealing score multipliers. A CFR approach could potentially learn this strategy, but will need to encode an enormous amount of actions.

Thus, we have several options for representing an actions:

Let's define a move as a sequence of (char, (row, col)) tuples:

1. Look into the theoretical sound-ness of chaining multiple actions together and still using the same CFR update rule
    - Leads to a maximum of 15x15x(26 + 1) possible actions (I think)
    - Might have to revisit seq2seq update rules for inspiration here
2. Create a mapping between the closest word that will be created by applying a move and the move sequence itself
    - Maximum number of moves is tied to the vocabulary size (in the words.txt we have now this is almost 250k)
3. Shrink the vocabulary by stemming the words and apply additional post-processing to recover the original word given a valid move
    - At worst case could have as many moves as option 2
4. Completely ignore the generated word and rely on the move generator to order the words by score. We then simply use the AI
purely for strategy that involves cutting other players off based on the length of the word it chooses.
    
    - Each action then becomes a choice between the row/col/length of the word to place. 
    - This roughly equates to a maximum of 15x15x5 actions per state since we must form a word of length [2,7] on each turn 