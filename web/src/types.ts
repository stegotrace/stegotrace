export type Severity = "info" | "low" | "medium" | "high";

export interface Finding {
  id: string;
  category: string;
  title: string;
  severity: Severity;
  method: string;
  value: unknown;
  interpretation: string;
  confidence: number;
}

export interface Artifact {
  id: string;
  kind: string;
  suggested_name: string;
  size: number;
  sha256: string;
  description: string;
  mime: string;
}

export interface Report {
  schema_version: string;
  engine_version: string;
  filename: string;
  media_type: string;
  size: number;
  sha256: string;
  verdict: string;
  score: number;
  score_kind: string;
  findings: Finding[];
  artifacts: Artifact[];
  scientific: {
    available: boolean;
    provider: string;
    methods: string[];
    predictions: Record<string, number>;
    limitation: string | null;
  };
  methods: string[];
  limitations: string[];
}

