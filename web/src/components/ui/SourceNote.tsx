import { FileCheck2 } from 'lucide-react';

export function SourceNote({ text }: { text: string }) {
  return (
    <p className="mt-4 inline-flex items-start gap-2 rounded-lg border border-white/10 bg-white/4 px-3 py-2 text-xs text-gray-400">
      <FileCheck2 className="mt-0.5 h-3.5 w-3.5 shrink-0 text-blue-300" />
      <span>{text}</span>
    </p>
  );
}
