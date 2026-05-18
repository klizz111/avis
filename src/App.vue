<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";

type ApiEnvelope<T> = {
  ok: boolean;
  data: T | null;
  error: string | null;
};

type GeneratedProof = {
  message_text: string;
  message_b64: string;
  share_b64: string;
  pk_share_b64: string;
  r_b64: string;
  s_b64: string;
  seed: number;
  nonce: number;
  timestamp: number;
};

type ProofPayload = {
  R: string;
  s: string;
  pk_share: string;
  nonce: number;
  ts: number;
};

type ProofVerifyData = {
  valid: boolean;
  reason: string | null;
};

type PartialSignatureData = {
  sigma?: string;
};

type AggregateData = {
  signature?: string;
  verified?: boolean;
  threshold_public_key_b64?: string | null;
  partial_count?: number;
};

type DemoNodeData = {
  node_id: string;
  commitments_b64: string[];
  share_to_user_b64: string;
  final_share_b64: string;
  public_share_b64: string;
};

type DemoRoundData = {
  round_id: string;
  threshold: number;
  participants: string[];
  user_index: number;
  user_seed: number;
  user_share_b64: string;
  user_pk_share_b64: string;
  threshold_public_key_b64: string;
  nodes: DemoNodeData[];
};

type DkgInitData = {
  status: string;
  round_id: string;
  threshold: number;
  participants: string[];
  note: string;
  demo_round?: DemoRoundData;
};

type DemoRunData = {
  status: string;
  round_id: string;
  threshold: number;
  partial_count: number;
  partial_signatures: string[];
  signature: string;
  verified: boolean;
  threshold_public_key_b64: string;
  proof_valid: boolean;
  user_seed: number;
  reason?: string;
};

type DemoStepStatus = "idle" | "running" | "done" | "error";

type DemoStep = {
  key: string;
  title: string;
  description: string;
  status: DemoStepStatus;
  detail: string;
};

const apiBaseUrl = ref(localStorage.getItem("avis.apiBaseUrl") || "http://127.0.0.1:8443/api/v1");
const proofMessageText = ref("threshold signature demo");
const shareSeed = ref(7);
const nonceValue = ref(Date.now() % 1_000_000);
const timestampValue = ref(Math.floor(Date.now() / 1000));
const generatedProof = ref<GeneratedProof | null>(null);

const serviceHealth = ref("No request yet.");
const knownNodes = ref("No request yet.");
const dkgInitResult = ref("No request yet.");
const commitmentResult = ref("No request yet.");
const shareResult = ref("No request yet.");
const proofVerifyResult = ref("No request yet.");
const partialSignResult = ref("No request yet.");
const aggregateResult = ref("No request yet.");
const demoStatus = ref("Idle.");
const demoLog = ref("No demo run yet.");
const demoRunning = ref(false);
const activeDemoRound = ref<DemoRoundData | null>(null);
const demoSteps = reactive<DemoStep[]>([
  {
    key: "health",
    title: "Service health",
    description: "Check that the API gateway is reachable.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "nodes",
    title: "Node discovery",
    description: "Read the mock node list before dispatching work.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "dkg",
    title: "DKG bootstrap",
    description: "Submit the round parameters and get the session accepted.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "proof",
    title: "Schnorr proof",
    description: "Generate a local proof for the threshold share.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "partial",
    title: "Partial signing",
    description: "Ask a node to issue a partial signature after proof check.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "aggregate",
    title: "Signature aggregation",
    description: "Combine partial signatures and verify the final result.",
    status: "idle",
    detail: "Waiting to start.",
  },
]);

const dkgRoundId = ref("round-001");
const dkgThreshold = ref(2);
const dkgParticipants = ref("node-a\nnode-b\nnode-c");

const commitmentRoundId = ref("round-001");
const commitmentNodeId = ref("node-a");
const commitmentList = ref("");

const shareRoundId = ref("round-001");
const shareFromNode = ref("node-a");
const shareToNode = ref("node-b");
const shareToIndex = ref(2);
const shareValue = ref("");
const shareCommitments = ref("");

const partialNodeId = ref("node-a");
const partialMessageText = ref("threshold signature demo");
const partialShareB64 = ref("");
const partialProofR = ref("");
const partialProofS = ref("");
const partialProofPkShare = ref("");
const partialProofNonce = ref(Date.now() % 1_000_000);
const partialProofTimestamp = ref(Math.floor(Date.now() / 1000));

const aggregateMessageText = ref("threshold signature demo");
const aggregatePartialSignatures = ref("");

const proofPayload = computed<ProofPayload>(() => ({
  R: partialProofR.value,
  s: partialProofS.value,
  pk_share: partialProofPkShare.value,
  nonce: Number(partialProofNonce.value),
  ts: Number(partialProofTimestamp.value),
}));

function normalizeBaseUrl(value: string): string {
  return value.replace(/\/+$/, "");
}

function textToBase64(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
}

function formatJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

function parseEnvelope<T>(value: string): ApiEnvelope<T> | null {
  try {
    return JSON.parse(value) as ApiEnvelope<T>;
  } catch {
    return null;
  }
}

function appendDemoLog(line: string) {
  demoLog.value = demoLog.value ? `${demoLog.value}\n${line}` : line;
}

function resetDemoSteps() {
  demoSteps.forEach((step) => {
    step.status = "idle";
    step.detail = "Waiting to start.";
  });
}

function updateDemoStep(key: string, status: DemoStepStatus, detail: string) {
  const step = demoSteps.find((item) => item.key === key);
  if (!step) {
    return;
  }
  step.status = status;
  step.detail = detail;
}

function applyDemoRound(round: DemoRoundData) {
  activeDemoRound.value = round;
  dkgRoundId.value = round.round_id;
  dkgThreshold.value = round.threshold;
  dkgParticipants.value = round.participants.join("\n");

  commitmentRoundId.value = round.round_id;
  commitmentNodeId.value = round.participants[0] || commitmentNodeId.value;
  commitmentList.value = round.nodes[0]?.commitments_b64.join("\n") || "";

  shareRoundId.value = round.round_id;
  shareFromNode.value = round.nodes[0]?.node_id || shareFromNode.value;
  shareToNode.value = round.participants[round.user_index - 1] || shareToNode.value;
  shareToIndex.value = round.user_index;
  shareValue.value = round.nodes[0]?.share_to_user_b64 || "";
  shareCommitments.value = round.nodes[0]?.commitments_b64.join("\n") || "";

  partialNodeId.value = round.participants[round.user_index - 1] || partialNodeId.value;
  partialShareB64.value = round.user_share_b64;
  partialProofPkShare.value = round.user_pk_share_b64;

  aggregateMessageText.value = proofMessageText.value;
  aggregatePartialSignatures.value = "";
}

async function postJson<T>(path: string, body: unknown): Promise<ApiEnvelope<T>> {
  try {
    const response = await fetch(`${normalizeBaseUrl(apiBaseUrl.value)}${path}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    });
    return (await response.json()) as ApiEnvelope<T>;
  } catch (error) {
    return {
      ok: false,
      data: null,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

async function getJson<T>(path: string): Promise<ApiEnvelope<T>> {
  try {
    const response = await fetch(`${normalizeBaseUrl(apiBaseUrl.value)}${path}`);
    return (await response.json()) as ApiEnvelope<T>;
  } catch (error) {
    return {
      ok: false,
      data: null,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

async function checkHealth() {
  serviceHealth.value = formatJson(await getJson("/healthz"));
}

async function refreshNodes() {
  knownNodes.value = formatJson(await getJson("/nodes"));
}

async function submitDkgInit() {
  const participants = dkgParticipants.value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);

  const response = await postJson<DkgInitData>("/dkg/init", {
      round_id: dkgRoundId.value,
      threshold: Number(dkgThreshold.value),
      participants,
    });

  dkgInitResult.value = formatJson(response);
  if (response.ok && response.data?.demo_round) {
    applyDemoRound(response.data.demo_round);
  }

  return response;
}

async function requestDemoProof() {
  return postJson<GeneratedProof>("/demo/proof", {
    round_id: activeDemoRound.value?.round_id || dkgRoundId.value,
    message: proofMessageText.value,
    seed: Number(shareSeed.value),
    nonce: Number(nonceValue.value),
    timestamp: Number(timestampValue.value),
  });
}

async function submitCommitments() {
  const commitments = commitmentList.value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);

  commitmentResult.value = formatJson(
    await postJson("/dkg/commitment/submit", {
      round_id: commitmentRoundId.value,
      node_id: commitmentNodeId.value,
      commitments,
    }),
  );
}

async function submitShare() {
  const commitments = shareCommitments.value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);

  shareResult.value = formatJson(
    await postJson("/dkg/share/submit", {
      round_id: shareRoundId.value,
      from_node: shareFromNode.value,
      to_node: shareToNode.value,
      to_index: Number(shareToIndex.value),
      share: shareValue.value,
      commitments,
    }),
  );
}

function syncGeneratedProof(bundle: GeneratedProof) {
  proofMessageText.value = bundle.message_text;
  shareSeed.value = bundle.seed;
  partialMessageText.value = bundle.message_text;
  partialShareB64.value = bundle.share_b64;
  partialProofR.value = bundle.r_b64;
  partialProofS.value = bundle.s_b64;
  partialProofPkShare.value = bundle.pk_share_b64;
  partialProofNonce.value = bundle.nonce;
  partialProofTimestamp.value = bundle.timestamp;
  aggregateMessageText.value = bundle.message_text;
}

async function generateProof() {
  const response = await requestDemoProof();
  if (!response.ok || !response.data) {
    throw new Error(response.error || "proof generation failed");
  }

  generatedProof.value = response.data;
  syncGeneratedProof(response.data);
  return response.data;
}

async function verifyProof() {
  const response = await postJson<ProofVerifyData>("/proof/verify", {
    message: textToBase64(partialMessageText.value),
    proof: proofPayload.value,
  });
  proofVerifyResult.value = formatJson(response);
  return response;
}

async function requestPartialSignature() {
  const response = await postJson<PartialSignatureData>("/sign/partial", {
    node_id: partialNodeId.value,
    message: textToBase64(partialMessageText.value),
    share: partialShareB64.value,
    proof: proofPayload.value,
  });
  partialSignResult.value = formatJson(response);

  try {
    const payload = response;
    const signature = payload.data?.sigma;
    if (typeof signature === "string" && signature.trim().length > 0) {
      aggregatePartialSignatures.value = aggregatePartialSignatures.value
        ? `${aggregatePartialSignatures.value.trim()}\n${signature}`
        : signature;
    }
  } catch {
    // Keep the response visible even if parsing fails.
  }

  return response;
}

async function aggregateSignatures() {
  const partials = aggregatePartialSignatures.value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);

  const response = await postJson<AggregateData>("/bls/aggregate", {
    message: textToBase64(aggregateMessageText.value),
    partial_signatures: partials,
  });
  aggregateResult.value = formatJson(response);
  return response;
}

async function runDemoSignAndAggregate() {
  const response = await postJson<DemoRunData>("/demo/run", {
    round_id: activeDemoRound.value?.round_id || dkgRoundId.value,
    message: textToBase64(partialMessageText.value),
    proof: proofPayload.value,
  });

  partialSignResult.value = formatJson(response);

  if (response.ok && response.data) {
    aggregatePartialSignatures.value = response.data.partial_signatures.join("\n");
    aggregateResult.value = formatJson({
      ok: true,
      data: {
        status: response.data.status,
        signature: response.data.signature,
        verified: response.data.verified,
        partial_count: response.data.partial_count,
        threshold_public_key_b64: response.data.threshold_public_key_b64,
      },
      error: null,
    });
  }

  return response;
}

async function runFullDemo() {
  demoRunning.value = true;
  demoStatus.value = "Running complete demo...";
  demoLog.value = "";
  aggregatePartialSignatures.value = "";
  resetDemoSteps();

  try {
    appendDemoLog("1. Checking API health.");
    updateDemoStep("health", "running", "Sending health check request.");
    await checkHealth();
    const health = parseEnvelope<unknown>(serviceHealth.value);
    if (!health?.ok) {
      updateDemoStep("health", "error", health?.error || "health check failed");
      throw new Error(health?.error || "health check failed");
    }
    updateDemoStep("health", "done", "API gateway responded with ok.");
    appendDemoLog("   Health check passed.");

    appendDemoLog("2. Refreshing node list.");
    updateDemoStep("nodes", "running", "Requesting the current mock node list.");
    await refreshNodes();
    const nodes = parseEnvelope<unknown>(knownNodes.value);
    if (!nodes?.ok) {
      updateDemoStep("nodes", "error", nodes?.error || "node discovery failed");
      throw new Error(nodes?.error || "node discovery failed");
    }
    updateDemoStep("nodes", "done", "Mock MPC nodes were discovered.");
    appendDemoLog("   Node discovery passed.");

    appendDemoLog("3. Initializing DKG round.");
    updateDemoStep("dkg", "running", "Submitting round id, threshold, and participants.");
    const dkgResponse = await submitDkgInit();
    const dkg = dkgResponse;
    if (!dkg?.ok) {
      updateDemoStep("dkg", "error", dkg?.error || "DKG bootstrap failed");
      throw new Error(dkg?.error || "DKG bootstrap failed");
    }
    updateDemoStep("dkg", "done", "DKG round accepted by the API.");
    appendDemoLog("   DKG bootstrap accepted.");

    appendDemoLog("4. Generating local Schnorr proof.");
    updateDemoStep("proof", "running", "Creating proof from the demo share seed.");
    const proofResponse = await requestDemoProof();
    if (!proofResponse.ok || !proofResponse.data) {
      updateDemoStep("proof", "error", proofResponse.error || "proof generation failed");
      throw new Error(proofResponse.error || "proof generation failed");
    }
    generatedProof.value = proofResponse.data;
    syncGeneratedProof(proofResponse.data);
    if (!generatedProof.value) {
      updateDemoStep("proof", "error", "proof generation failed");
      throw new Error("proof generation failed");
    }
    updateDemoStep("proof", "done", "Local proof and share bundle were generated.");
    appendDemoLog("   Proof bundle generated.");

    appendDemoLog("5. Verifying the proof against the API.");
    updateDemoStep("proof", "running", "Verifying the proof payload with the backend.");
    await verifyProof();
    const proof = parseEnvelope<ProofVerifyData>(proofVerifyResult.value);
    if (!proof?.ok || !proof.data?.valid) {
      updateDemoStep("proof", "error", proof?.data?.reason || proof?.error || "proof verification failed");
      throw new Error(proof?.data?.reason || proof?.error || "proof verification failed");
    }
    updateDemoStep("proof", "done", "Schnorr verification passed.");
    appendDemoLog("   Schnorr verification passed.");

    appendDemoLog("6. Asking the backend to sign and aggregate the active demo round.");
    updateDemoStep("partial", "running", "Backend is issuing threshold partial signatures.");
    updateDemoStep("aggregate", "running", "Backend is aggregating and verifying the final signature.");
    await runDemoSignAndAggregate();
    const demoRun = parseEnvelope<DemoRunData>(partialSignResult.value);
    if (!demoRun?.ok || !demoRun.data?.verified || !demoRun.data?.signature) {
      updateDemoStep("partial", "error", demoRun?.error || demoRun?.data?.reason || "demo signing failed");
      updateDemoStep("aggregate", "error", demoRun?.error || demoRun?.data?.reason || "demo aggregation failed");
      throw new Error(demoRun?.error || demoRun?.data?.reason || "demo signing failed");
    }
    updateDemoStep("partial", "done", `Backend returned ${demoRun.data.partial_count} partial signatures.`);
    updateDemoStep("aggregate", "done", "Aggregate signature verified by the backend.");
    appendDemoLog("   Backend demo run completed and verified.");

    demoStatus.value = "Demo completed successfully.";
  } catch (error) {
    demoStatus.value = `Demo failed: ${error instanceof Error ? error.message : String(error)}`;
    appendDemoLog(demoStatus.value);
  } finally {
    demoRunning.value = false;
  }
}

function adoptGeneratedProof() {
  if (generatedProof.value) {
    syncGeneratedProof(generatedProof.value);
  }
}

onMounted(() => {
  const savedBaseUrl = localStorage.getItem("avis.apiBaseUrl");
  if (savedBaseUrl) {
    apiBaseUrl.value = savedBaseUrl;
  }
});

watch(apiBaseUrl, (value) => {
  localStorage.setItem("avis.apiBaseUrl", value);
});
</script>

<template>
  <main class="shell">
    <section class="hero card">
      <div>
        <p class="eyebrow">Tauri threshold signer</p>
        <h1>Proof generation, partial signing, and BLS aggregation in one workspace.</h1>
        <p class="hero-copy">
          Generate a local Schnorr proof from a demo share seed, verify it against the example API,
          and push the resulting partial signatures into the aggregation endpoint.
        </p>
      </div>
      <div class="hero-meta">
        <div class="metric">
          <span>API base</span>
          <strong>{{ normalizeBaseUrl(apiBaseUrl) }}</strong>
        </div>
        <div class="metric">
          <span>Proof state</span>
          <strong>{{ generatedProof ? "ready" : "idle" }}</strong>
        </div>
        <div class="metric">
          <span>Demo status</span>
          <strong>{{ demoStatus }}</strong>
        </div>
      </div>
    </section>

    <section class="card panel demo-panel">
      <div class="panel-head">
        <div>
          <p class="panel-kicker">Quick demo</p>
          <h2>Run the full visible workflow in one click</h2>
        </div>
      </div>
      <p class="hero-copy demo-copy">
        This button reuses the existing frontend fields and walks through health check, DKG bootstrap,
        proof generation, proof verification, partial signing, and aggregation in sequence.
      </p>
      <div class="actions">
        <button type="button" @click="runFullDemo" :disabled="demoRunning">Run full demo</button>
      </div>
      <div class="step-grid">
        <article v-for="step in demoSteps" :key="step.key" class="step-card" :class="step.status">
          <div class="step-card-head">
            <div>
              <p class="step-label">{{ step.key }}</p>
              <h3>{{ step.title }}</h3>
            </div>
            <span class="step-badge">{{ step.status }}</span>
          </div>
          <p class="step-copy">{{ step.description }}</p>
          <pre>{{ step.detail }}</pre>
        </article>
      </div>
      <pre class="demo-log">{{ demoLog }}</pre>
    </section>

    <section class="grid two-up">
      <article class="card panel">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Connection</p>
            <h2>Service and node discovery</h2>
          </div>
        </div>
        <label class="field">
          <span>API base URL</span>
          <input v-model="apiBaseUrl" type="text" spellcheck="false" />
        </label>
        <div class="actions">
          <button type="button" @click="checkHealth">Check health</button>
          <button type="button" class="secondary" @click="refreshNodes">Refresh nodes</button>
        </div>
        <div class="stack">
          <details open>
            <summary>Health response</summary>
            <pre>{{ serviceHealth }}</pre>
          </details>
          <details>
            <summary>Nodes response</summary>
            <pre>{{ knownNodes }}</pre>
          </details>
        </div>
      </article>

      <article class="card panel">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">DKG</p>
            <h2>Round bootstrap and share checks</h2>
          </div>
        </div>
        <div class="grid form-grid">
          <label class="field wide">
            <span>Round ID</span>
            <input v-model="dkgRoundId" type="text" />
          </label>
          <label class="field">
            <span>Threshold</span>
            <input v-model="dkgThreshold" type="number" min="1" />
          </label>
          <label class="field wide">
            <span>Participants, one per line</span>
            <textarea v-model="dkgParticipants" rows="4" />
          </label>
        </div>
        <div class="actions">
          <button type="button" @click="submitDkgInit">Start round</button>
        </div>
        <pre>{{ dkgInitResult }}</pre>
      </article>
    </section>

    <section class="grid two-up">
      <article class="card panel">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Feldman</p>
            <h2>Commitment and share submission</h2>
          </div>
        </div>
        <div class="grid form-grid">
          <label class="field">
            <span>Round ID</span>
            <input v-model="commitmentRoundId" type="text" />
          </label>
          <label class="field">
            <span>Node ID</span>
            <input v-model="commitmentNodeId" type="text" />
          </label>
          <label class="field wide">
            <span>Commitment base64 values</span>
            <textarea v-model="commitmentList" rows="4" placeholder="One base64 commitment per line" />
          </label>
        </div>
        <div class="actions">
          <button type="button" @click="submitCommitments">Submit commitments</button>
        </div>
        <pre>{{ commitmentResult }}</pre>
      </article>

      <article class="card panel">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Share check</p>
            <h2>VSS verification</h2>
          </div>
        </div>
        <div class="grid form-grid">
          <label class="field">
            <span>Round ID</span>
            <input v-model="shareRoundId" type="text" />
          </label>
          <label class="field">
            <span>From node</span>
            <input v-model="shareFromNode" type="text" />
          </label>
          <label class="field">
            <span>To node</span>
            <input v-model="shareToNode" type="text" />
          </label>
          <label class="field">
            <span>Receiver index</span>
            <input v-model="shareToIndex" type="number" min="1" />
          </label>
          <label class="field wide">
            <span>Share base64</span>
            <textarea v-model="shareValue" rows="2" placeholder="Canonical scalar bytes in base64" />
          </label>
          <label class="field wide">
            <span>Commitments base64</span>
            <textarea v-model="shareCommitments" rows="4" placeholder="One base64 commitment per line" />
          </label>
        </div>
        <div class="actions">
          <button type="button" @click="submitShare">Verify share</button>
        </div>
        <pre>{{ shareResult }}</pre>
      </article>
    </section>

    <section class="grid two-up">
      <article class="card panel accent">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Proof builder</p>
            <h2>Generate and inspect a local Schnorr proof</h2>
          </div>
        </div>
        <div class="grid form-grid">
          <label class="field wide">
            <span>Message</span>
            <textarea v-model="proofMessageText" rows="3" />
          </label>
          <label class="field">
            <span>Share seed</span>
            <input v-model="shareSeed" type="number" min="0" />
          </label>
          <label class="field">
            <span>Nonce</span>
            <input v-model="nonceValue" type="number" min="0" />
          </label>
          <label class="field">
            <span>Timestamp</span>
            <input v-model="timestampValue" type="number" min="0" />
          </label>
        </div>
        <div class="actions">
          <button type="button" @click="generateProof">Generate proof</button>
          <button type="button" class="secondary" @click="adoptGeneratedProof" :disabled="!generatedProof">Reuse proof</button>
        </div>
        <pre>{{ generatedProof ? formatJson(generatedProof) : 'No proof generated yet.' }}</pre>
      </article>

      <article class="card panel">
        <div class="panel-head">
          <div>
            <p class="panel-kicker">Proof action</p>
            <h2>Send proof and request a partial signature</h2>
          </div>
        </div>
        <div class="grid form-grid">
          <label class="field">
            <span>Node ID</span>
            <input v-model="partialNodeId" type="text" />
          </label>
          <label class="field wide">
            <span>Message</span>
            <textarea v-model="partialMessageText" rows="3" />
          </label>
          <label class="field wide">
            <span>Share base64</span>
            <textarea v-model="partialShareB64" rows="2" />
          </label>
          <label class="field wide">
            <span>Proof R base64</span>
            <textarea v-model="partialProofR" rows="2" />
          </label>
          <label class="field wide">
            <span>Proof s base64</span>
            <textarea v-model="partialProofS" rows="2" />
          </label>
          <label class="field wide">
            <span>Proof pk_share base64</span>
            <textarea v-model="partialProofPkShare" rows="2" />
          </label>
          <label class="field">
            <span>Nonce</span>
            <input v-model="partialProofNonce" type="number" min="0" />
          </label>
          <label class="field">
            <span>Timestamp</span>
            <input v-model="partialProofTimestamp" type="number" min="0" />
          </label>
        </div>
        <div class="actions">
          <button type="button" @click="verifyProof">Verify proof</button>
          <button type="button" class="secondary" @click="requestPartialSignature">Request partial signature</button>
        </div>
        <pre>{{ proofVerifyResult }}</pre>
        <pre>{{ partialSignResult }}</pre>
      </article>
    </section>

    <section class="card panel">
      <div class="panel-head">
        <div>
          <p class="panel-kicker">Aggregation</p>
          <h2>Combine partial signatures into one BLS aggregate</h2>
        </div>
      </div>
      <div class="grid form-grid">
        <label class="field wide">
          <span>Message</span>
          <textarea v-model="aggregateMessageText" rows="3" />
        </label>
        <label class="field wide">
          <span>Partial signatures base64</span>
          <textarea v-model="aggregatePartialSignatures" rows="5" placeholder="One base64 G2 signature per line" />
        </label>
      </div>
      <div class="actions">
        <button type="button" @click="aggregateSignatures">Aggregate signatures</button>
      </div>
      <pre>{{ aggregateResult }}</pre>
    </section>
  </main>
</template>

<style scoped>
:global(:root) {
  font-family: "Avenir Next", "Segoe UI Variable", "SF Pro Display", "Helvetica Neue", sans-serif;
  color: #12212f;
  background:
    radial-gradient(circle at top left, rgba(245, 179, 92, 0.32), transparent 30%),
    radial-gradient(circle at top right, rgba(52, 134, 166, 0.22), transparent 26%),
    linear-gradient(180deg, #f7f1e8 0%, #eef3f6 48%, #e9eef4 100%);
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

:global(body) {
  margin: 0;
  min-width: 320px;
  min-height: 100vh;
}

:global(*) {
  box-sizing: border-box;
}

.shell {
  width: min(1240px, calc(100vw - 32px));
  margin: 0 auto;
  padding: 32px 0 48px;
}

.grid {
  display: grid;
  gap: 18px;
}

.two-up {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin-top: 18px;
}

.card {
  background: rgba(255, 255, 255, 0.74);
  backdrop-filter: blur(18px);
  border: 1px solid rgba(24, 39, 57, 0.08);
  border-radius: 24px;
  box-shadow: 0 20px 60px rgba(24, 39, 57, 0.09);
}

.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.3fr) minmax(280px, 0.7fr);
  gap: 24px;
  padding: 28px;
  align-items: end;
}

.eyebrow,
.panel-kicker {
  margin: 0 0 8px;
  text-transform: uppercase;
  letter-spacing: 0.18em;
  font-size: 0.76rem;
  color: #6a5a34;
}

h1,
h2,
p {
  margin-top: 0;
}

h1 {
  font-size: clamp(2rem, 4vw, 4rem);
  line-height: 1.04;
  margin-bottom: 14px;
  max-width: 12ch;
}

.hero-copy {
  max-width: 66ch;
  font-size: 1.02rem;
  line-height: 1.65;
  color: rgba(18, 33, 47, 0.8);
}

.hero-meta {
  display: grid;
  gap: 12px;
}

.metric {
  padding: 16px 18px;
  border-radius: 20px;
  background: linear-gradient(180deg, rgba(19, 38, 58, 0.92), rgba(20, 48, 63, 0.84));
  color: #fff;
}

.metric span {
  display: block;
  font-size: 0.82rem;
  opacity: 0.76;
  margin-bottom: 6px;
}

.metric strong {
  font-size: 1rem;
  line-height: 1.45;
  word-break: break-word;
}

.panel {
  padding: 22px;
}

.panel-head {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.panel h2 {
  margin-bottom: 0;
  font-size: 1.18rem;
}

.form-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.wide {
  grid-column: 1 / -1;
}

.field {
  display: grid;
  gap: 8px;
}

.field span {
  font-size: 0.88rem;
  color: rgba(18, 33, 47, 0.78);
}

input,
textarea,
button {
  font: inherit;
}

input,
textarea {
  width: 100%;
  border-radius: 16px;
  border: 1px solid rgba(27, 43, 61, 0.12);
  background: rgba(255, 255, 255, 0.9);
  color: #12212f;
  padding: 12px 14px;
  outline: none;
  transition: border-color 0.18s ease, box-shadow 0.18s ease, transform 0.18s ease;
}

textarea {
  resize: vertical;
  min-height: 78px;
}

input:focus,
textarea:focus {
  border-color: rgba(40, 107, 138, 0.52);
  box-shadow: 0 0 0 4px rgba(40, 107, 138, 0.12);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin: 16px 0 12px;
}

button {
  border: 0;
  border-radius: 999px;
  padding: 11px 16px;
  color: #fff;
  background: linear-gradient(135deg, #1f6f8b, #15344b);
  box-shadow: 0 10px 24px rgba(21, 52, 75, 0.22);
  cursor: pointer;
}

button.secondary {
  color: #183246;
  background: rgba(255, 255, 255, 0.86);
  border: 1px solid rgba(24, 50, 70, 0.15);
  box-shadow: none;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.56;
}

pre {
  margin: 0;
  padding: 14px;
  border-radius: 18px;
  background: rgba(13, 25, 38, 0.94);
  color: #f5f7fa;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  min-height: 96px;
}

.stack {
  display: grid;
  gap: 12px;
}

details {
  border-radius: 18px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.58);
  border: 1px solid rgba(24, 39, 57, 0.08);
}

summary {
  cursor: pointer;
  padding: 12px 14px;
  font-weight: 700;
  color: #183246;
}

details pre {
  border-radius: 0 0 18px 18px;
  min-height: 0;
}

.accent {
  background:
    radial-gradient(circle at top right, rgba(245, 179, 92, 0.18), transparent 26%),
    rgba(255, 255, 255, 0.74);
}

.demo-panel {
  margin-top: 18px;
}

.demo-copy {
  margin-bottom: 14px;
}

.step-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
  margin-bottom: 14px;
}

.step-card {
  padding: 16px;
  border-radius: 20px;
  border: 1px solid rgba(24, 39, 57, 0.1);
  background: rgba(255, 255, 255, 0.82);
  display: grid;
  gap: 10px;
  min-height: 100%;
}

.step-card.running {
  border-color: rgba(245, 179, 92, 0.48);
  box-shadow: inset 0 0 0 1px rgba(245, 179, 92, 0.18);
}

.step-card.done {
  border-color: rgba(60, 145, 118, 0.35);
  background: linear-gradient(180deg, rgba(240, 252, 247, 0.96), rgba(255, 255, 255, 0.88));
}

.step-card.error {
  border-color: rgba(183, 64, 54, 0.42);
  background: linear-gradient(180deg, rgba(255, 241, 239, 0.96), rgba(255, 255, 255, 0.88));
}

.step-card-head {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 12px;
}

.step-label {
  margin: 0 0 4px;
  font-size: 0.72rem;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: rgba(18, 33, 47, 0.52);
}

.step-card h3 {
  margin: 0;
  font-size: 1rem;
  line-height: 1.35;
}

.step-copy {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.55;
  color: rgba(18, 33, 47, 0.76);
}

.step-badge {
  flex: 0 0 auto;
  border-radius: 999px;
  padding: 6px 10px;
  font-size: 0.72rem;
  line-height: 1;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  background: rgba(24, 50, 70, 0.08);
  color: #183246;
}

.step-card.running .step-badge {
  background: rgba(245, 179, 92, 0.18);
  color: #8a5a00;
}

.step-card.done .step-badge {
  background: rgba(60, 145, 118, 0.15);
  color: #2f715b;
}

.step-card.error .step-badge {
  background: rgba(183, 64, 54, 0.16);
  color: #9e352e;
}

.demo-log {
  min-height: 126px;
}

@media (max-width: 1080px) {
  .hero,
  .two-up {
    grid-template-columns: 1fr;
  }

  .step-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .shell {
    width: min(100vw - 20px, 1240px);
    padding-top: 16px;
  }

  .panel,
  .hero {
    padding: 18px;
  }

  .step-grid,
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>