#!/usr/bin/env python3
from argparse import ArgumentParser
from typing import List, Dict, Sequence
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import scipy.stats as st

from blau import BlauState, BlauAgent


def main():
    ap = ArgumentParser()
    ap.add_argument('--num_players', type=int, default=4)
    ap.add_argument('--num_games', type=int, default=100)
    ap.add_argument('--plot', action='store_true')
    ap.add_argument('--debug', action='store_true')
    ap.add_argument('--base_level', type=int, default=1)
    ap.add_argument('--test_level', type=int, default=2)
    args = ap.parse_args()

    levels = [args.test_level] + [args.base_level] * (args.num_players - 1)
    agents = [BlauAgent(level) for level in levels]
    agent_names = ['Test', 'B', 'C', 'D'][:args.num_players]

    scores = play_games(agents, agent_names, args.num_games)
    print('Per game scoring')
    print(scores.describe())
    if args.plot:
        fig, axes = plt.subplots(ncols=3, figsize=(15, 5))
        axes[0].violinplot(scores, showextrema=False, widths=0.8)
        scores.plot(kind='box', ax=axes[0], xlabel='AI Player', ylabel='Score')
        axes[0].set_xlabel('AI Player')

    # compute rankings, see https://www.berkayantmen.com/rank
    ranks = scores.copy()
    ranks[:] = args.num_players - scores.values.argsort().argsort()
    print('\nPer game ranking (1 = winner)')
    print(ranks.describe())
    if args.plot:
        expected = args.num_games // args.num_players
        n = args.num_players + 1
        hist = pd.DataFrame(
            {
                name: np.bincount(ranks[name], minlength=n)[1:] - expected
                for name in agent_names
            },
            index=np.arange(1, n))
        axes[1].axhline(expected, c='k', ls='--')
        hist.plot(kind='bar',
                  ax=axes[1],
                  legend=True,
                  xlabel='Rank',
                  ylabel='Frequency',
                  bottom=expected)

    # compute p-values for test outperforming the others
    p_values = np.array([
        st.wilcoxon(scores.Test - scores[name],
                    zero_method='pratt',
                    alternative='greater',
                    mode='approx').pvalue for name in agent_names[1:]
    ])
    print('\np-Values:', p_values)
    if (p_values < 0.05).all():
        print('Test condition is an improvement!')
    elif (p_values > 0.95).all():
        print('Test condition is a regression!')
    else:
        print('Test condition is inconclusive.')

    # compute running Elo ratings
    k = 20
    elos = np.zeros((len(scores) + 1, args.num_players))
    elos[0] = 1500 + np.arange(args.num_players)
    for i, s in enumerate(scores.values):
        elos[i + 1] = elos[i] + update_elos(elos[i], s.argsort(), k)
        k = max(0.99 * k, 1)
    print('\nFinal Elo ratings:')
    for elo, name in sorted(zip(elos[-1], agent_names), reverse=True):
        print(f'{name}: {elo:.2f}')
    if args.plot:
        lines = axes[2].plot(elos)
        axes[2].legend(lines, agent_names)
        axes[2].set_xlabel('Games Played')
        axes[2].set_ylabel('Elo rating')
        plt.tight_layout()
        plt.show()

    if args.debug:
        import IPython
        IPython.embed()


def play_games(agents: List[BlauAgent], agent_names: List[str],
               num_games: int) -> pd.DataFrame:
    agent_scores = {n: [] for n in agent_names}  # type: Dict[str, List[int]]
    for _ in range(num_games):
        game = BlauState(agent_names)
        while True:
            game.start_round()
            while True:
                ai = agents[game.curr_player_idx]
                move = ai.choose_action(game)
                if game.do_move(move):
                    break
            if game.finish_round():
                break
        for name, score in game.players():
            agent_scores[name].append(score)
    return pd.DataFrame(agent_scores)


def _elo_change(rating_loser: float, rating_winner: float, k: float):
    gap = rating_winner - rating_loser
    return k / (1 + 10**(gap / 400))


def update_elos(current_elos: Sequence[float], ranking: Sequence[int],
                elo_k: float) -> Sequence[float]:
    num_players = len(current_elos)
    elo_change = [0.0] * num_players
    for j in range(1, num_players):
        idx_loser = ranking[j - 1]
        idx_winner = ranking[j]
        delta = _elo_change(current_elos[idx_loser], current_elos[idx_winner],
                            elo_k)
        elo_change[idx_loser] -= delta
        elo_change[idx_winner] += delta
    return elo_change


if __name__ == "__main__":
    main()
