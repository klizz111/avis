<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";

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

type DemoStepStatus = "idle" | "running" | "done" | "error";

type DemoStep = {
  key: string;
  title: string;
  description: string;
  status: DemoStepStatus;
  detail: string;
};

type ActiveView = "onboarding" | "console";

type SignatureAuditDecision = {
  node_id: string;
  approved: boolean;
  reason: string;
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
const partialNodeIdSecondary = ref("node-b");
const partialMessageText = ref("threshold signature demo");
const partialShareB64 = ref("");
const partialProofR = ref("");
const partialProofS = ref("");
const partialProofPkShare = ref("");
const partialProofNonce = ref(Date.now() % 1_000_000);
const partialProofTimestamp = ref(Math.floor(Date.now() / 1000));

const aggregateMessageText = ref("threshold signature demo");
const aggregatePartialSignatures = ref("");
const signatureMessageText = ref("Audit release: all nodes approve the signing request.");
const signatureAuditReport = ref("No signature audit has run yet.");
const signatureVerificationReport = ref("No signature verification has run yet.");
const lastAggregatedSignature = ref("No aggregated signature yet.");
const signatureFlowStatus = ref("Idle.");
const activeView = ref<ActiveView>("onboarding");
const cameraVideoRef = ref<HTMLVideoElement | null>(null);
const cameraCanvasRef = ref<HTMLCanvasElement | null>(null);
const cameraStream = ref<MediaStream | null>(null);
const cameraStatus = ref("Camera idle.");
const cameraFingerprint = ref("No biometric sample captured yet.");
const registeredAccountId = ref("account-001");
const registeredDisplayName = ref("Demo User");
const registrationRoundId = ref("round-001");
const registrationThreshold = ref(2);
const registrationParticipants = ref("node-a\nnode-b\nnode-c");
const biometricEnrollment = ref("");
const biometricVerification = ref("");
const biometricScore = ref<number | null>(null);
const onboardingStatus = ref("Idle.");
const onboardingLog = ref("No onboarding run yet.");
const onboardingRunning = ref(false);
const onboardingSteps = reactive<DemoStep[]>([
  {
    key: "camera",
    title: "Camera capture",
    description: "Open the webcam and collect a biometric-like sample for the session.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "register",
    title: "Account registration",
    description: "Register the account metadata and bootstrap a DKG round on the backend.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "verify",
    title: "Biometric re-check",
    description: "Capture again and compare the new sample against the enrolled one.",
    status: "idle",
    detail: "Waiting to start.",
  },
  {
    key: "sign",
    title: "Proof and signing",
    description: "Generate Schnorr proof, request partial signatures, and aggregate them.",
    status: "idle",
    detail: "Waiting to start.",
  },
]);

const proofPayload = computed<ProofPayload>(() => ({
  R: partialProofR.value,
  s: partialProofS.value,
  pk_share: partialProofPkShare.value,
  nonce: Number(partialProofNonce.value),
  ts: Number(partialProofTimestamp.value),
}));

const aggregateVerified = computed(() => {
  const parsed = parseEnvelope<AggregateData>(aggregateResult.value);
  return Boolean(parsed?.ok && parsed.data?.verified);
});

const allowedSignatureMessage = "Audit release: all nodes approve the signing request.";
const blockedSignatureMessage = "Audit release: node-b and node-c reject this signing request.";

// const activeStatusText = computed(() => (activeView.value === "onboarding" ? onboardingStatus.value : demoStatus.value));

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

function appendOnboardingLog(line: string) {
  onboardingLog.value = onboardingLog.value ? `${onboardingLog.value}\n${line}` : line;
}

function syncSignatureMessage(message: string) {
  signatureMessageText.value = message;
  proofMessageText.value = message;
  partialMessageText.value = message;
  aggregateMessageText.value = message;
}

function applySignaturePreset(kind: "allow" | "block") {
  syncSignatureMessage(kind === "allow" ? allowedSignatureMessage : blockedSignatureMessage);
  signatureFlowStatus.value = kind === "allow" ? "Loaded the all-nodes-approve message." : "Loaded the node-b/node-c rejection message.";
  signatureAuditReport.value = "Preset selected. Run audited signing to inspect decisions.";
  signatureVerificationReport.value = "No signature verification has run yet.";
}

function auditSignatureRequest(nodeId: string, message: string): SignatureAuditDecision {
  if (message === blockedSignatureMessage && (nodeId === "node-b" || nodeId === "node-c")) {
    return {
      node_id: nodeId,
      approved: false,
      reason: "frontend audit rule: node-b and node-c reject this message",
    };
  }

  if (message === allowedSignatureMessage) {
    return {
      node_id: nodeId,
      approved: true,
      reason: "frontend audit rule: message accepted by all nodes",
    };
  }

  return {
    node_id: nodeId,
    approved: true,
    reason: "frontend audit rule: custom message treated as approved",
  };
}

function collectAuditedNodeIds(): string[] {
  const nodes = activeDemoRound.value?.participants?.length
    ? activeDemoRound.value.participants
    : dkgParticipants.value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean);

  return Array.from(new Set(nodes.length > 0 ? nodes : ["node-a", "node-b", "node-c"]));
}

async function runAuditedSignatureFlow() {
  signatureFlowStatus.value = "Running audited signing flow...";
  signatureAuditReport.value = "";
  signatureVerificationReport.value = "";
  lastAggregatedSignature.value = "No aggregated signature yet.";
  aggregatePartialSignatures.value = "";
  aggregateResult.value = "No aggregate has been generated yet.";

  try {
    syncSignatureMessage(signatureMessageText.value || allowedSignatureMessage);

    const proofResponse = await generateProof();
    if (!proofResponse) {
      throw new Error("proof generation failed");
    }

    const proofResponseCheck = await verifyProof();
    const proofResult = parseEnvelope<ProofVerifyData>(proofVerifyResult.value);
    if (!proofResponseCheck.ok || !proofResult?.ok || !proofResult.data?.valid) {
      throw new Error(proofResult?.data?.reason || proofResult?.error || proofResponseCheck.error || "proof verification failed");
    }

    const auditedNodeIds = collectAuditedNodeIds();
    const auditDecisions = auditedNodeIds.map((nodeId) => auditSignatureRequest(nodeId, signatureMessageText.value));
    signatureAuditReport.value = formatJson({
      message: signatureMessageText.value,
      auditDecisions,
    });

    const approvedNodes = auditDecisions.filter((decision) => decision.approved).map((decision) => decision.node_id);
    if (approvedNodes.length < Number(dkgThreshold.value)) {
      const auditError = {
        ok: false,
        data: {
          approved_nodes: approvedNodes,
          required_threshold: Number(dkgThreshold.value),
        },
        error: "Audit rejected enough nodes that a full aggregate signature cannot be produced.",
      };
      partialSignResult.value = formatJson(auditError);
      aggregateResult.value = formatJson(auditError);
      lastAggregatedSignature.value = "No complete signature could be generated because the audit blocked too many nodes.";
      signatureVerificationReport.value = "Skipped: insufficient approved nodes for full aggregation.";
      signatureFlowStatus.value = "Audit blocked the signing flow before full aggregation.";
      return;
    }

    const collectedSignatures: string[] = [];
    const perNodeResults: Array<{ node_id: string; ok: boolean; sigma?: string; error?: string | null }> = [];

    for (const nodeId of approvedNodes) {
      const response = await requestPartialSignatureForNode(nodeId);
      const sigma = response.data?.sigma;
      if (typeof sigma === "string" && sigma.trim().length > 0) {
        collectedSignatures.push(sigma);
      }
      perNodeResults.push({
        node_id: nodeId,
        ok: response.ok,
        sigma: typeof sigma === "string" ? sigma : undefined,
        error: response.error,
      });
    }

    aggregatePartialSignatures.value = collectedSignatures.join("\n");
    partialSignResult.value = formatJson({
      ok: true,
      data: {
        requested_nodes: approvedNodes,
        collected_count: collectedSignatures.length,
        results: perNodeResults,
      },
      error: null,
    });

    const aggregateResponse = await aggregateSignatures();
    if (!aggregateResponse.ok || !aggregateResponse.data?.verified || !aggregateResponse.data?.signature) {
      throw new Error(aggregateResponse.error || "aggregate verification failed");
    }

    lastAggregatedSignature.value = aggregateResponse.data.signature;
    signatureVerificationReport.value = formatJson(aggregateResponse);
    signatureFlowStatus.value = "Audited signing completed and the aggregate signature was verified.";
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    signatureFlowStatus.value = `Audited signing failed: ${message}`;
    signatureVerificationReport.value = message;
  }
}

async function verifyLastAggregatedSignature() {
  if (!aggregatePartialSignatures.value.trim()) {
    signatureVerificationReport.value = "No partial signatures are available to verify.";
    return;
  }

  const response = await aggregateSignatures();
  signatureVerificationReport.value = formatJson(response);
  if (response.ok && response.data?.signature) {
    lastAggregatedSignature.value = response.data.signature;
  }
}

function resetDemoSteps() {
  demoSteps.forEach((step) => {
    step.status = "idle";
    step.detail = "Waiting to start.";
  });
}

function resetOnboardingSteps() {
  onboardingSteps.forEach((step) => {
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

function updateOnboardingStep(key: string, status: DemoStepStatus, detail: string) {
  const step = onboardingSteps.find((item) => item.key === key);
  if (!step) {
    return;
  }
  step.status = status;
  step.detail = detail;
}

function stopCameraPreview() {
  const stream = cameraStream.value;
  if (!stream) {
    return;
  }

  stream.getTracks().forEach((track) => track.stop());
  cameraStream.value = null;

  if (cameraVideoRef.value) {
    cameraVideoRef.value.srcObject = null;
  }
}

async function startCameraPreview() {
  if (!navigator.mediaDevices?.getUserMedia) {
    cameraStatus.value = "Camera API unavailable, falling back to simulated capture.";
    return false;
  }

  stopCameraPreview();

  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: "user" },
      audio: false,
    });
    cameraStream.value = stream;
    if (cameraVideoRef.value) {
      cameraVideoRef.value.srcObject = stream;
      await cameraVideoRef.value.play().catch(() => undefined);
    }
    cameraStatus.value = "Camera preview is active.";
    return true;
  } catch (error) {
    cameraStatus.value = `Camera unavailable, using simulated capture: ${error instanceof Error ? error.message : String(error)}`;
    return false;
  }
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
}

function encodeFingerprintSource(source: string): string {
  const bytes = new TextEncoder().encode(source);
  return bytesToBase64(bytes);
}

function base64ToBytes(value: string): Uint8Array | null {
  try {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      bytes[index] = binary.charCodeAt(index);
    }
    return bytes;
  } catch {
    return null;
  }
}

async function captureBiometricFingerprint(label: string): Promise<string> {
  const video = cameraVideoRef.value;
  const canvas = cameraCanvasRef.value;

  if (!video || !canvas || video.videoWidth === 0 || video.videoHeight === 0) {
    return encodeFingerprintSource(`${registeredAccountId.value}|${registrationRoundId.value}|${registeredDisplayName.value}|${label}`);
  }

  const width = 24;
  const height = 24;
  canvas.width = width;
  canvas.height = height;

  const context = canvas.getContext("2d");
  if (!context) {
    return encodeFingerprintSource(`${registeredAccountId.value}|${registrationRoundId.value}|${label}`);
  }

  context.drawImage(video, 0, 0, width, height);
  const imageData = context.getImageData(0, 0, width, height).data;
  const collapsed: number[] = [];
  const blockSize = 6;

  for (let y = 0; y < height; y += blockSize) {
    for (let x = 0; x < width; x += blockSize) {
      let total = 0;
      let count = 0;

      for (let innerY = y; innerY < Math.min(y + blockSize, height); innerY += 1) {
        for (let innerX = x; innerX < Math.min(x + blockSize, width); innerX += 1) {
          const offset = (innerY * width + innerX) * 4;
          const red = imageData[offset] ?? 0;
          const green = imageData[offset + 1] ?? 0;
          const blue = imageData[offset + 2] ?? 0;
          total += Math.round((red + green + blue) / 3);
          count += 1;
        }
      }

      collapsed.push(Math.round(total / Math.max(count, 1)));
    }
  }

  return bytesToBase64(new Uint8Array(collapsed));
}

function fingerprintSimilarity(left: string, right: string): number {
  if (!left || !right) {
    return 0;
  }

  const leftBytes = base64ToBytes(left) ?? new TextEncoder().encode(left);
  const rightBytes = base64ToBytes(right) ?? new TextEncoder().encode(right);
  const length = Math.min(leftBytes.length, rightBytes.length);

  if (length === 0) {
    return 0;
  }

  let totalDifference = 0;
  for (let index = 0; index < length; index += 1) {
    totalDifference += Math.abs((leftBytes[index] ?? 0) - (rightBytes[index] ?? 0));
  }

  const averageDifference = totalDifference / length;
  const score = 100 - Math.round((averageDifference / 255) * 100);
  return Math.max(0, Math.min(100, score));
}

async function captureEnrollmentSample() {
  updateOnboardingStep("camera", "running", "Capturing the enrollment sample.");
  await startCameraPreview();
  biometricEnrollment.value = await captureBiometricFingerprint("enroll");
  cameraFingerprint.value = biometricEnrollment.value;
  cameraStatus.value = `Enrollment sample captured (${biometricEnrollment.value.length} chars).`;
  updateOnboardingStep("camera", "done", "Enrollment sample was captured from the camera or fallback simulation.");
  appendOnboardingLog("Camera enrollment sample captured.");
}

async function captureVerificationSample() {
  updateOnboardingStep("verify", "running", "Re-capturing biometric data for verification.");
  biometricVerification.value = await captureBiometricFingerprint("verify");
  cameraFingerprint.value = biometricVerification.value;
  biometricScore.value = fingerprintSimilarity(biometricEnrollment.value, biometricVerification.value);
  const approved = biometricScore.value >= 55;
  cameraStatus.value = `Biometric verification ${approved ? "passed" : "needs another try"} with score ${biometricScore.value}%.`;
  updateOnboardingStep(
    "verify",
    approved ? "done" : "error",
    approved ? `Verification passed with score ${biometricScore.value}%` : `Verification score ${biometricScore.value}% is below the demo threshold`,
  );
  appendOnboardingLog(`Biometric verification score: ${biometricScore.value}%.`);
  if (!approved) {
    throw new Error("biometric verification did not meet the demo threshold");
  }
}

async function submitRegistrationDkgInit() {
  updateOnboardingStep("register", "running", "Sending account metadata and DKG parameters to the backend.");

  const participants = registrationParticipants.value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);

  const response = await postJson<DkgInitData>("/dkg/init", {
    round_id: registrationRoundId.value,
    threshold: Number(registrationThreshold.value),
    participants,
  });

  dkgInitResult.value = formatJson(response);
  if (!response.ok || !response.data?.demo_round) {
    updateOnboardingStep("register", "error", response.error || "registration and DKG bootstrap failed");
    throw new Error(response.error || "registration and DKG bootstrap failed");
  }

  applyDemoRound(response.data.demo_round);
  onboardingStatus.value = `Account ${registeredAccountId.value} registered and DKG round ${registrationRoundId.value} is active.`;
  updateOnboardingStep("register", "done", `Round ${registrationRoundId.value} accepted by the backend.`);
  appendOnboardingLog(`Backend accepted registration for ${registeredAccountId.value}.`);
  return response;
}

async function runOnboardingJourney() {
  onboardingRunning.value = true;
  onboardingStatus.value = "Running onboarding journey...";
  onboardingLog.value = "";
  biometricEnrollment.value = "";
  biometricVerification.value = "";
  biometricScore.value = null;
  resetOnboardingSteps();

  try {
    appendOnboardingLog("1. Opening the camera and collecting a registration sample.");
    await captureEnrollmentSample();

    appendOnboardingLog("2. Registering the account and bootstrapping the DKG round.");
    await submitRegistrationDkgInit();

    appendOnboardingLog("3. Re-checking the biometric sample before signing.");
    await captureVerificationSample();

    appendOnboardingLog("4. Generating a Schnorr proof and requesting a legal threshold signature.");
    updateOnboardingStep("sign", "running", "Generating proof, verifying it, and requesting partial signatures.");
    const proofResponse = await generateProof();
    if (!proofResponse) {
      throw new Error("proof generation failed");
    }

    const proofResponseCheck = await verifyProof();
    const proofResult = parseEnvelope<ProofVerifyData>(proofVerifyResult.value);
    if (!proofResponseCheck.ok || !proofResult?.ok || !proofResult.data?.valid) {
      throw new Error(proofResult?.data?.reason || proofResult?.error || proofResponseCheck.error || "proof verification failed");
    }

    await requestTwoPartialSignatures();
    const aggregateResponse = parseEnvelope<AggregateData>(aggregateResult.value);
    if (!aggregateResponse?.ok || !aggregateResponse.data?.verified || !aggregateResponse.data?.signature) {
      throw new Error(aggregateResponse?.error || "aggregate verification failed");
    }

    updateOnboardingStep("sign", "done", "Proof verification and aggregate signature verification passed.");
    onboardingStatus.value = "Onboarding flow completed successfully with a verified aggregate signature.";
    appendOnboardingLog("Proof verification and aggregate signature verification passed.");
  } catch (error) {
    onboardingStatus.value = `Onboarding flow failed: ${error instanceof Error ? error.message : String(error)}`;
    appendOnboardingLog(onboardingStatus.value);
    const signStep = onboardingSteps.find((step) => step.key === "sign");
    if (signStep?.status === "running") {
      updateOnboardingStep("sign", "error", error instanceof Error ? error.message : String(error));
    }
  } finally {
    onboardingRunning.value = false;
  }
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
  partialNodeIdSecondary.value = round.participants[0] || partialNodeIdSecondary.value;
  if (partialNodeIdSecondary.value === partialNodeId.value) {
    partialNodeIdSecondary.value = round.participants[1] || partialNodeIdSecondary.value;
  }
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

async function requestPartialSignatureForNode(nodeId: string) {
  const response = await postJson<PartialSignatureData>("/sign/partial", {
    node_id: nodeId,
    round_id: activeDemoRound.value?.round_id || dkgRoundId.value,
    message: textToBase64(partialMessageText.value),
    share: partialShareB64.value,
    proof: proofPayload.value,
  });

  return response;
}

async function requestPartialSignature() {
  const response = await requestPartialSignatureForNode(partialNodeId.value);
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

async function requestTwoPartialSignatures() {
  const nodeIds = [partialNodeId.value.trim(), partialNodeIdSecondary.value.trim()]
    .filter((value) => value.length > 0)
    .filter((value, index, arr) => arr.indexOf(value) === index);

  if (nodeIds.length < 2) {
    partialSignResult.value = formatJson({
      ok: false,
      data: null,
      error: "Please provide two different node IDs.",
    });
    return;
  }

  const collectedSignatures: string[] = [];
  const perNodeResults: Array<{ node_id: string; ok: boolean; sigma?: string; error?: string | null }> = [];

  for (const nodeId of nodeIds) {
    const response = await requestPartialSignatureForNode(nodeId);
    const sigma = response.data?.sigma;
    if (typeof sigma === "string" && sigma.trim().length > 0) {
      collectedSignatures.push(sigma);
    }
    perNodeResults.push({
      node_id: nodeId,
      ok: response.ok,
      sigma: typeof sigma === "string" ? sigma : undefined,
      error: response.error,
    });
  }

  aggregatePartialSignatures.value = collectedSignatures.join("\n");

  const allOk = perNodeResults.every((item) => item.ok && typeof item.sigma === "string" && item.sigma.length > 0);
  partialSignResult.value = formatJson({
    ok: allOk,
    data: {
      requested_nodes: nodeIds,
      collected_count: collectedSignatures.length,
      results: perNodeResults,
    },
    error: allOk ? null : "One or more partial signatures failed.",
  });

  if (collectedSignatures.length >= 2) {
    await aggregateSignatures();
  }
}

async function aggregateSignatures() {
  const partials = aggregatePartialSignatures.value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);

  const response = await postJson<AggregateData>("/bls/aggregate", {
    round_id: activeDemoRound.value?.round_id || dkgRoundId.value,
    message: textToBase64(aggregateMessageText.value),
    partial_signatures: partials,
  });
  aggregateResult.value = formatJson(response);
  if (response.ok && response.data?.signature) {
    lastAggregatedSignature.value = response.data.signature;
  }
  signatureVerificationReport.value = formatJson(response);
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

    appendDemoLog("6. Requesting two partial signatures, then aggregating on /bls/aggregate.");
    updateDemoStep("partial", "running", "Calling /sign/partial for two selected nodes.");
    updateDemoStep("aggregate", "running", "Calling /bls/aggregate with collected partial signatures.");
    await requestTwoPartialSignatures();

    const partialResponse = parseEnvelope<{ collected_count?: number }>(partialSignResult.value);
    if (!partialResponse?.ok) {
      updateDemoStep("partial", "error", partialResponse?.error || "partial signing failed");
      updateDemoStep("aggregate", "error", "aggregation skipped because partial signing failed");
      throw new Error(partialResponse?.error || "partial signing failed");
    }

    const aggregateResponse = parseEnvelope<AggregateData>(aggregateResult.value);
    if (!aggregateResponse?.ok || !aggregateResponse.data?.verified || !aggregateResponse.data?.signature) {
      updateDemoStep("partial", "done", `Collected ${partialResponse.data?.collected_count ?? 0} partial signatures.`);
      updateDemoStep("aggregate", "error", aggregateResponse?.error || "aggregate verification failed");
      throw new Error(aggregateResponse?.error || "aggregate verification failed");
    }

    updateDemoStep("partial", "done", `Collected ${partialResponse.data?.collected_count ?? 0} partial signatures.`);
    updateDemoStep("aggregate", "done", "Aggregate signature verified by /bls/aggregate.");
    appendDemoLog("   /sign/partial and /bls/aggregate completed with verification passed.");

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

onUnmounted(() => {
  stopCameraPreview();
});

watch(apiBaseUrl, (value) => {
  localStorage.setItem("avis.apiBaseUrl", value);
});

watch(signatureMessageText, (value) => {
  proofMessageText.value = value;
  partialMessageText.value = value;
  aggregateMessageText.value = value;
});
</script>

<template>
  <main class="shell">
    <section class="hero card">
      <div>
        <h1>Avis</h1>
      </div>
    </section>

    <section class="card panel view-switch-panel">
      <div class="view-switch-head">
        <div>
          <p class="panel-kicker">Interface switch</p>
          <h2>Choose onboarding interface or legacy console</h2>
        </div>
      </div>
      <div class="view-switch-actions">
        <button
          type="button"
          class="switch-button"
          :class="{ active: activeView === 'onboarding' }"
          @click="activeView = 'onboarding'"
        >
          New onboarding interface
        </button>
        <button
          type="button"
          class="switch-button secondary"
          :class="{ active: activeView === 'console' }"
          @click="activeView = 'console'"
        >
          Legacy console
        </button>
      </div>
    </section>

    <template v-if="activeView === 'onboarding'">

    <section class="card panel onboarding-panel">
      <div class="panel-head">
        <div>
          <p class="panel-kicker">New interface</p>
          <h2>Camera enrollment, backend registration, and biometric re-check</h2>
        </div>
      </div>
      <p class="hero-copy demo-copy">
        This workflow simulates camera collection first, registers the account through the backend DKG round,
        then asks for a second biometric check before generating Schnorr proof material and requesting a valid signature.
      </p>

      <div class="grid onboarding-grid">
        <div class="camera-card">
          <video ref="cameraVideoRef" class="camera-feed" autoplay playsinline muted></video>
          <canvas ref="cameraCanvasRef" class="camera-canvas" aria-hidden="true"></canvas>
          <div class="camera-summary">
            <div>
              <span>Camera state</span>
              <strong>{{ cameraStatus }}</strong>
            </div>
            <div>
              <span>Biometric sample</span>
              <strong>{{ cameraFingerprint }}</strong>
            </div>
            <div>
              <span>Match score</span>
              <strong>{{ biometricScore === null ? 'No verification yet' : `${biometricScore}%` }}</strong>
            </div>
          </div>
        </div>

        <div class="stack">
          <div class="grid form-grid">
            <label class="field">
              <span>Account ID</span>
              <input v-model="registeredAccountId" type="text" />
            </label>
            <label class="field">
              <span>Display name</span>
              <input v-model="registeredDisplayName" type="text" />
            </label>
            <label class="field">
              <span>Round ID</span>
              <input v-model="registrationRoundId" type="text" />
            </label>
            <label class="field">
              <span>Threshold</span>
              <input v-model="registrationThreshold" type="number" min="1" />
            </label>
            <label class="field wide">
              <span>Participants, one per line</span>
              <textarea v-model="registrationParticipants" rows="4" />
            </label>
          </div>

          <div class="actions">
            <button type="button" @click="startCameraPreview">Enable camera</button>
            <button type="button" class="secondary" @click="captureEnrollmentSample">Capture enrollment sample</button>
            <button type="button" class="secondary" @click="captureVerificationSample">Capture verification sample</button>
            <button type="button" @click="runOnboardingJourney" :disabled="onboardingRunning">Run onboarding journey</button>
          </div>

          <div class="step-grid onboarding-step-grid">
            <article v-for="step in onboardingSteps" :key="step.key" class="step-card" :class="step.status">
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

          <div class="stack onboarding-summary">
            <details open>
              <summary>Onboarding status</summary>
              <pre>{{ onboardingStatus }}</pre>
            </details>
            <details>
              <summary>Biometric samples</summary>
              <pre>{{ formatJson({ enrollment: biometricEnrollment, verification: biometricVerification }) }}</pre>
            </details>
            <details>
              <summary>Onboarding log</summary>
              <pre class="demo-log">{{ onboardingLog }}</pre>
            </details>
          </div>

          <section class="card panel onboarding-signature-panel">
            <div class="panel-head">
              <div>
                <p class="panel-kicker">Signature business logic</p>
                <h2>Custom signature audit, aggregation, and verification</h2>
              </div>
            </div>
            <p class="hero-copy demo-copy">
              The onboarding interface now includes the same signing workflow: choose a custom message, apply a preset audit rule,
              generate signatures, and verify the last aggregate directly in this flow.
            </p>
            <div class="grid form-grid">
              <label class="field wide">
                <span>Custom signature message</span>
                <textarea v-model="signatureMessageText" rows="3" placeholder="Describe the signing request here" />
              </label>
            </div>
            <div class="actions">
              <button type="button" @click="applySignaturePreset('allow')">Load all-nodes-approved message</button>
              <button type="button" class="secondary" @click="applySignaturePreset('block')">Load node-b/node-c rejected message</button>
              <button type="button" @click="runAuditedSignatureFlow">Run audited signing</button>
              <button type="button" class="secondary" @click="verifyLastAggregatedSignature">Verify signature</button>
            </div>
            <div class="stack">
              <details open>
                <summary>Audit report</summary>
                <pre>{{ signatureAuditReport }}</pre>
              </details>
              <details open>
                <summary>Last aggregated signature</summary>
                <pre>{{ lastAggregatedSignature }}</pre>
              </details>
              <details>
                <summary>Signature verification</summary>
                <pre>{{ signatureVerificationReport }}</pre>
              </details>
              <details>
                <summary>Signature workflow status</summary>
                <pre>{{ signatureFlowStatus }}</pre>
              </details>
            </div>
            <p v-if="aggregateVerified" class="success-banner">Aggregate verification passed.</p>
            <div class="grid form-grid">
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
        </div>
      </div>
    </section>

    </template>

    <template v-else>

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
            <span>Node ID (primary)</span>
            <input v-model="partialNodeId" type="text" />
          </label>
          <label class="field">
            <span>Node ID (secondary)</span>
            <input v-model="partialNodeIdSecondary" type="text" />
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
          <button type="button" class="secondary" @click="requestTwoPartialSignatures">Request two partial signatures</button>
        </div>
        <pre>{{ proofVerifyResult }}</pre>
        <pre>{{ partialSignResult }}</pre>
      </article>
    </section>

    <section class="card panel" :class="{ 'aggregate-success': aggregateVerified }">
      <div class="panel-head">
        <div>
          <p class="panel-kicker">Aggregation</p>
          <h2>Custom signature audit, aggregation, and verification</h2>
        </div>
      </div>
      <p class="hero-copy demo-copy">
        Use the two preset messages to simulate a message that all nodes accept or a message that node-b and node-c reject.
        The audit happens in the frontend before any partial signature request is sent.
      </p>
      <div class="grid form-grid">
        <label class="field wide">
          <span>Custom signature message</span>
          <textarea v-model="signatureMessageText" rows="3" placeholder="Describe the signing request here" />
        </label>
      </div>
      <div class="actions">
        <button type="button" @click="applySignaturePreset('allow')">Load all-nodes-approved message</button>
        <button type="button" class="secondary" @click="applySignaturePreset('block')">Load node-b/node-c rejected message</button>
        <button type="button" @click="runAuditedSignatureFlow">Run audited signing</button>
        <button type="button" class="secondary" @click="verifyLastAggregatedSignature">Verify signature</button>
      </div>
      <div class="stack">
        <details open>
          <summary>Audit report</summary>
          <pre>{{ signatureAuditReport }}</pre>
        </details>
        <details open>
          <summary>Last aggregated signature</summary>
          <pre>{{ lastAggregatedSignature }}</pre>
        </details>
        <details>
          <summary>Signature verification</summary>
          <pre>{{ signatureVerificationReport }}</pre>
        </details>
        <details>
          <summary>Signature workflow status</summary>
          <pre>{{ signatureFlowStatus }}</pre>
        </details>
      </div>
      <p v-if="aggregateVerified" class="success-banner">Aggregate verification passed.</p>
      <div class="grid form-grid">
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

    </template>
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

.view-switch-panel {
  margin-top: 18px;
}

.view-switch-head {
  margin-bottom: 12px;
}

.view-switch-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.switch-button {
  min-width: 220px;
}

.switch-button.active {
  box-shadow: 0 0 0 3px rgba(40, 107, 138, 0.2);
  transform: translateY(-1px);
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

.onboarding-panel {
  margin-top: 18px;
  border: 1px solid rgba(37, 112, 143, 0.18);
  background:
    radial-gradient(circle at top left, rgba(75, 146, 170, 0.14), transparent 30%),
    radial-gradient(circle at top right, rgba(245, 179, 92, 0.12), transparent 26%),
    rgba(255, 255, 255, 0.76);
}

.onboarding-grid {
  grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
  gap: 18px;
}

.camera-card {
  display: grid;
  gap: 14px;
  align-content: start;
}

.camera-feed {
  width: 100%;
  min-height: 320px;
  border-radius: 22px;
  background: linear-gradient(180deg, rgba(19, 38, 58, 0.92), rgba(20, 48, 63, 0.84));
  object-fit: cover;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
}

.camera-canvas {
  display: none;
}

.camera-summary {
  display: grid;
  gap: 12px;
}

.camera-summary > div {
  padding: 14px 16px;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.88);
  border: 1px solid rgba(24, 39, 57, 0.08);
}

.camera-summary span {
  display: block;
  margin-bottom: 6px;
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: rgba(18, 33, 47, 0.56);
}

.camera-summary strong {
  display: block;
  word-break: break-word;
  color: #12212f;
  line-height: 1.45;
}

.onboarding-step-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.onboarding-summary {
  margin-top: 4px;
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

.aggregate-success {
  border-color: rgba(32, 150, 107, 0.45);
  box-shadow:
    0 20px 60px rgba(24, 39, 57, 0.09),
    0 0 0 2px rgba(32, 150, 107, 0.18) inset,
    0 0 0 6px rgba(32, 150, 107, 0.08);
}

.success-banner {
  margin: 0 0 12px;
  padding: 10px 12px;
  border-radius: 12px;
  font-weight: 700;
  color: #176247;
  background: linear-gradient(135deg, rgba(151, 255, 221, 0.42), rgba(217, 255, 240, 0.82));
  border: 1px solid rgba(32, 150, 107, 0.32);
  animation: glowPulse 1.2s ease-in-out 2;
}

@keyframes glowPulse {
  0% {
    box-shadow: 0 0 0 0 rgba(32, 150, 107, 0.28);
  }
  70% {
    box-shadow: 0 0 0 10px rgba(32, 150, 107, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(32, 150, 107, 0);
  }
}

@media (max-width: 1080px) {
  .hero,
  .two-up {
    grid-template-columns: 1fr;
  }

  .onboarding-grid,
  .onboarding-step-grid {
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