import { getContributorLeaderboard } from '@/lib/api';
import { Avatar } from '@/components/ui/Avatar';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { SourceNote } from '@/components/ui/SourceNote';

export default async function LeaderboardPage() {
  const contributors = await getContributorLeaderboard();

  return (
    <div className="mx-auto max-w-5xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Contributor Leaderboard</h1>
      <p className="mt-1 text-sm text-gray-400">Top contributors ranked by reputation and verified evidence quality.</p>

      <Card className="mt-6">
        <CardHeader><CardTitle>Top contributors</CardTitle></CardHeader>
        <CardContent className="space-y-3">
          {contributors.map((person, index) => (
            <div key={person.id} className="flex items-center gap-3 rounded-xl border border-white/10 bg-white/3 p-3">
              <div className="w-6 text-sm font-semibold text-blue-300">#{index + 1}</div>
              <Avatar name={person.name} />
              <div className="flex-1">
                <p className="text-sm font-medium text-white">{person.name}</p>
                <p className="text-xs text-gray-400">Recent: {person.recentContribution}</p>
              </div>
              <div className="text-right text-xs text-gray-400">
                <p className="text-sm font-semibold text-white">{person.reputation.toLocaleString()}</p>
                <p>rep score</p>
              </div>
              <Badge variant={person.trustTier === 'Platinum' ? 'red' : person.trustTier === 'Gold' ? 'yellow' : person.trustTier === 'Silver' ? 'blue' : 'outline'}>
                {person.trustTier}
              </Badge>
            </div>
          ))}
        </CardContent>
      </Card>

      <SourceNote text="Source attribution: leaderboard metrics are derived from validated submission logs and source verification outcomes." />
    </div>
  );
}
