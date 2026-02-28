import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';

export default function DonatePage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Support Nonfaction</h1>
      <p className="mt-1 text-sm text-gray-400">Independent accountability infrastructure funded by public-interest supporters.</p>

      <div className="mt-6 grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader><CardTitle className="text-base">Mission funding</CardTitle></CardHeader>
          <CardContent className="text-sm text-gray-300">Support source ingestion, verification workflows, and public API maintenance.</CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle className="text-base">Transparency commitment</CardTitle></CardHeader>
          <CardContent className="text-sm text-gray-300">Funding reports and budget usage snapshots are published in platform updates.</CardContent>
        </Card>
        <Card>
          <CardHeader><CardTitle className="text-base">Donation options</CardTitle></CardHeader>
          <CardContent className="space-y-2 text-sm text-gray-300">
            <p>Individual donor portal (placeholder)</p>
            <p>Institutional sponsorship (placeholder)</p>
            <Button size="sm">Open donation flow</Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
