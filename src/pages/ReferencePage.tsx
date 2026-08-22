import reference from "../data/reference.json" with { type: "json" };
import { copy } from "@/lib/copy";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type Row = { name: string; perf: string; notes: string };
type Group = { name: string; rows: Row[] };

const data = reference as { note: string; groups: Group[] };

export function ReferencePage() {
  return (
    <div className="space-y-4">
      <p className="text-sm">{copy.refLead}</p>
      <p className="text-sm text-muted-foreground">{data.note}</p>
      {data.groups.map((group) => (
        <Card key={group.name}>
          <CardHeader>
            <CardTitle>{group.name}</CardTitle>
          </CardHeader>
          <CardContent>
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-1.5 pr-3 font-medium">Setting</th>
                  <th className="py-1.5 pr-3 font-medium">Performance first</th>
                  <th className="py-1.5 font-medium">Note</th>
                </tr>
              </thead>
              <tbody>
                {group.rows.map((row) => (
                  <tr key={row.name} className="border-b last:border-0">
                    <td className="py-1.5 pr-3">{row.name}</td>
                    <td className="py-1.5 pr-3">{row.perf}</td>
                    <td className="py-1.5 text-muted-foreground">{row.notes}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
