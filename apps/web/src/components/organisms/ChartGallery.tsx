import { Heading } from "@open-faktura/ui";
import { useState } from "react";
import { FilterTabs } from "@open-faktura/ui";
import { DataChart, type ChartPoint } from "@open-faktura/ui/charts";

const demo: ChartPoint[] = [
  { label: "Kvě", values: { issued: 24, paid: 20 } },
  { label: "Čvn", values: { issued: 32, paid: 25 } },
  { label: "Čvc", values: { issued: 28, paid: 28 } },
  { label: "Srp", values: { issued: 42, paid: 34 } },
  { label: "Zář", values: { issued: 36, paid: 31 } },
  { label: "Říj", values: { issued: 48, paid: 39 } },
];
const edgeCases: ChartPoint[] = [
  { label: "Kvě", values: { issued: 12, paid: 8 } },
  { label: "Čvn", values: { issued: 0, paid: 0 } },
  { label: "Čvc", values: { issued: null, paid: null } },
  { label: "Srp", values: { issued: -8, paid: -4 } },
  { label: "Zář", values: { issued: 16, paid: 12 } },
];
const series = [
  { key: "issued", label: "Vystavené" },
  { key: "paid", label: "Uhrazené" },
] as const;

export default function ChartGallery() {
  const [state, setState] = useState("demo");
  const data = state === "empty" ? [] : state === "edge" ? edgeCases : demo;
  return (
    <section aria-labelledby="charts-title" className="mt-8">
      <div className="mb-5">
        <Heading level={2} size="section" id="charts-title">
          Grafy a přehledy
        </Heading>
        <p className="mt-2 text-sm text-muted">
          Recharts · syntetická data pro ukázku vzhledu. Nuly, chybějící hodnoty
          i záporné hodnoty mají vlastní zobrazení.
        </p>
      </div>
      <div className="mb-5">
        <FilterTabs
          label="Ukázkové stavy grafů"
          value={state}
          onChange={setState}
          options={[
            { value: "demo", label: "Ukázková data" },
            { value: "empty", label: "Bez dat" },
            { value: "edge", label: "Hraniční hodnoty" },
            { value: "loading", label: "Načítání" },
            { value: "error", label: "Chyba načítání" },
          ]}
        />
      </div>
      <div className="grid gap-group xl:grid-cols-2">
        <DataChart
          title="Vývoj počtu faktur"
          description="Čárový graf pro porovnání vývoje v čase."
          data={data}
          loading={state === "loading"}
          error={
            state === "error"
              ? "Graf se nepodařilo načíst. Zkuste to znovu."
              : undefined
          }
          onRetry={() => setState("demo")}
          series={series}
        />
        <DataChart
          title="Faktury po měsících"
          description="Sloupcový graf pro porovnání jednotlivých období."
          data={data}
          loading={state === "loading"}
          error={
            state === "error"
              ? "Graf se nepodařilo načíst. Zkuste to znovu."
              : undefined
          }
          onRetry={() => setState("demo")}
          series={series}
          variant="bar"
        />
      </div>
      <p className="mt-3 text-xs text-muted">
        V režimu hraničních hodnot jsou záporné počty pouze technickou ukázkou.
        Graf lze procházet šipkami; přesná data jsou také v tabulce.
      </p>
    </section>
  );
}
