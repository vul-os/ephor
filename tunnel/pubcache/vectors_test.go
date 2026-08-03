package pubcache

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"testing"
)

// vectors_test.go — the GO half of the cross-language lock for FEEDS.md § 5.3.
//
// The problem this closes. § 5.3 has three implementations: this package, the
// browser verifier (kotva bindings/js/src/chunkProof.js), and the spec text
// FEEDS.md § 5.3 — which cites this package BY NAME as fielded evidence for a
// normative decision. Until this file existed the two code halves each asserted
// a HAND-COPIED constant (interopRootB64 / interopProofHex here, INTEROP_* over
// there) with nothing mechanically comparing them. Copies drift, and worse: a
// round-trip cannot see two implementations agreeing on something WRONG. Only
// foreign bytes can.
//
// testdata/chunkproof_vectors.json is those foreign bytes. It is produced by
// kotva conformance/vectors/gen_chunkproof_vectors.py, an independent Python
// generator that imports neither implementation and computes every value from
// the § 22.2.2 leaf/node formulas and the § 18.1.2 CBOR rules. Read its
// `generated_by` field: it is honest that it is not clean-room. What it buys is
// that Go and JS are each checked against a THIRD set of bytes rather than
// against each other, so a one-sided change to either goes red on its own.
//
// HOW THE TWO REPOS ARE BOUND, without either needing the other checked out.
// The canonical file lives in kotva (conformance/vectors/chunkproof_vectors.json);
// this is a byte-identical copy. Both suites assert the SAME sha256 of their own
// copy — chunkProofVectorsSHA256 below, CHUNKPROOF_VECTORS_SHA256 in
// bindings/js/test/chunkProofVectors.test.js. If the two copies ever diverge, or
// if either copy is edited to accommodate a broken implementation, one side's
// digest assertion fails. A pin needs no sibling checkout and no network, so
// unlike a cross-repo path it cannot degrade into a gate that skips itself and
// reports success. Regenerating the corpus is therefore a deliberate act that
// touches BOTH repos, which is exactly the property that was missing.
const chunkProofVectorsSHA256 = "2ab20686f293b3a142bd574640c141fa9163aec1eb7666ab6166d0142fa2ad22"

// Coverage floors. A harness that iterates nothing must not read as a pass, so
// the counts are asserted three ways: against these floors, against the
// corpus's own declared `counts`, and against what the loops actually executed.
const (
	minVectorCount  = 10
	minProofCount   = 60
	minControlCount = 8
)

type chunkProofCorpus struct {
	Format string `json:"format"`
	Counts struct {
		Vectors            int `json:"vectors"`
		Proofs             int `json:"proofs"`
		CorruptionControls int `json:"corruption_controls"`
	} `json:"counts"`
	Vectors []struct {
		Name          string   `json:"name"`
		N             int      `json:"n"`
		ChunksHex     []string `json:"chunks_hex"`
		ChunkAddrsHex []string `json:"chunk_addrs_hex"`
		RootHex       string   `json:"root_hex"`
		RootB64URL    string   `json:"root_b64url"`
		Proofs        []struct {
			Index        int      `json:"index"`
			PathHex      []string `json:"path_hex"`
			ProofBodyHex string   `json:"proof_body_hex"`
		} `json:"proofs"`
	} `json:"vectors"`
	CorruptionControls []struct {
		Name         string   `json:"name"`
		Surface      string   `json:"surface"`
		Defect       string   `json:"defect"`
		RootHex      string   `json:"root_hex"`
		NChunks      int      `json:"n_chunks"`
		Index        int      `json:"index"`
		ChunkHex     string   `json:"chunk_hex"`
		PathHex      []string `json:"path_hex"`
		ProofBodyHex string   `json:"proof_body_hex"`
		Expect       string   `json:"expect"`
	} `json:"corruption_controls"`
}

func mustHex(t *testing.T, s string) []byte {
	t.Helper()
	b, err := hex.DecodeString(s)
	if err != nil {
		t.Fatalf("corpus contains malformed hex %q: %v", s, err)
	}
	return b
}

func mustAddr(t *testing.T, s string) Addr {
	t.Helper()
	b := mustHex(t, s)
	if len(b) != addrLen {
		t.Fatalf("corpus address %q is %d bytes, want %d", s, len(b), addrLen)
	}
	var a Addr
	copy(a[:], b)
	return a
}

func loadChunkProofCorpus(t *testing.T) *chunkProofCorpus {
	t.Helper()
	raw, err := os.ReadFile("testdata/chunkproof_vectors.json")
	if err != nil {
		t.Fatalf("the shared corpus is missing — an absent subject is a FAILURE, not a skip: %v", err)
	}
	sum := sha256.Sum256(raw)
	if got := hex.EncodeToString(sum[:]); got != chunkProofVectorsSHA256 {
		t.Fatalf("corpus sha256 = %s, pinned %s\n"+
			"This copy and kotva's have diverged, or this one was edited. Fix the "+
			"IMPLEMENTATION, or regenerate the corpus in kotva and update the pin in BOTH "+
			"tunnel/pubcache/vectors_test.go and bindings/js/test/chunkProofVectors.test.js.",
			got, chunkProofVectorsSHA256)
	}
	var c chunkProofCorpus
	if err := json.Unmarshal(raw, &c); err != nil {
		t.Fatalf("corpus does not parse: %v", err)
	}
	if c.Format != "kotva-conformance-vectors/1" {
		t.Fatalf("corpus format %q, want kotva-conformance-vectors/1", c.Format)
	}
	if len(c.Vectors) != c.Counts.Vectors {
		t.Fatalf("corpus carries %d vectors but declares %d", len(c.Vectors), c.Counts.Vectors)
	}
	if len(c.CorruptionControls) != c.Counts.CorruptionControls {
		t.Fatalf("corpus carries %d controls but declares %d",
			len(c.CorruptionControls), c.Counts.CorruptionControls)
	}
	if len(c.Vectors) < minVectorCount {
		t.Fatalf("corpus has %d vectors, floor is %d", len(c.Vectors), minVectorCount)
	}
	return &c
}

// TestSharedVectorsRootsAndProofs is the positive half: every root and every
// proof body in the corpus is reproduced by THIS implementation, and every proof
// body AS WRITTEN IN THE CORPUS verifies against its root.
//
// Both directions matter and they are not the same check. Reproducing the bytes
// catches an encoder divergence; verifying the corpus's own bytes catches a
// verifier that only accepts what its own encoder produced.
func TestSharedVectorsRootsAndProofs(t *testing.T) {
	c := loadChunkProofCorpus(t)

	rootsChecked, proofsChecked, addrsChecked := 0, 0, 0
	for _, v := range c.Vectors {
		if len(v.ChunksHex) != v.N || len(v.ChunkAddrsHex) != v.N || len(v.Proofs) != v.N {
			t.Fatalf("%s: declares n=%d but carries %d chunks, %d addrs, %d proofs",
				v.Name, v.N, len(v.ChunksHex), len(v.ChunkAddrsHex), len(v.Proofs))
		}

		data := make([][]byte, v.N)
		chunks := make([]Addr, v.N)
		for i, h := range v.ChunksHex {
			data[i] = mustHex(t, h)
			chunks[i] = HashBytes(data[i])
			if got := hex.EncodeToString(chunks[i][:]); got != v.ChunkAddrsHex[i] {
				t.Errorf("%s chunk %d: address %s, corpus says %s", v.Name, i, got, v.ChunkAddrsHex[i])
			}
			addrsChecked++
		}

		wantRoot := mustAddr(t, v.RootHex)
		gotRoot := ManifestRoot(chunks)
		if gotRoot != wantRoot {
			t.Errorf("%s: root %s, corpus says %s — the tree, the DS tag or the leaf rule differs",
				v.Name, gotRoot, wantRoot)
		}
		if gotRoot.String() != v.RootB64URL {
			t.Errorf("%s: root base64url %s, corpus says %s", v.Name, gotRoot.String(), v.RootB64URL)
		}
		rootsChecked++

		for _, p := range v.Proofs {
			path, err := ChunkProof(chunks, p.Index)
			if err != nil {
				t.Fatalf("%s chunk %d: %v", v.Name, p.Index, err)
			}
			if len(path) != len(p.PathHex) {
				t.Errorf("%s chunk %d: path of %d elements, corpus says %d — promotion rule differs",
					v.Name, p.Index, len(path), len(p.PathHex))
			}
			if got := hex.EncodeToString(EncodeChunkProof(p.Index, path)); got != p.ProofBodyHex {
				t.Errorf("%s chunk %d proof body\n got %s\nwant %s", v.Name, p.Index, got, p.ProofBodyHex)
			}

			// The corpus's OWN bytes, decoded and folded — not this package's.
			idx, corpusPath, err := DecodeChunkProof(mustHex(t, p.ProofBodyHex))
			if err != nil {
				t.Fatalf("%s chunk %d: corpus proof body does not decode: %v", v.Name, p.Index, err)
			}
			if idx != p.Index {
				t.Errorf("%s chunk %d: decoded index %d", v.Name, p.Index, idx)
			}
			if err := VerifyChunkProof(wantRoot, v.N, p.Index, data[p.Index], corpusPath); err != nil {
				t.Errorf("%s chunk %d: corpus proof does not verify: %v", v.Name, p.Index, err)
			}
			proofsChecked++
		}
	}

	if rootsChecked == 0 || proofsChecked == 0 || addrsChecked == 0 {
		t.Fatalf("COVERAGE: exercised %d roots, %d proofs, %d addresses — an empty corpus "+
			"must not read as a pass", rootsChecked, proofsChecked, addrsChecked)
	}
	if rootsChecked != c.Counts.Vectors || proofsChecked != c.Counts.Proofs {
		t.Fatalf("COVERAGE: exercised %d roots / %d proofs, corpus declares %d / %d",
			rootsChecked, proofsChecked, c.Counts.Vectors, c.Counts.Proofs)
	}
	if rootsChecked < minVectorCount || proofsChecked < minProofCount {
		t.Fatalf("COVERAGE: %d roots (floor %d), %d proofs (floor %d)",
			rootsChecked, minVectorCount, proofsChecked, minProofCount)
	}
	t.Logf("shared corpus: %d roots, %d proofs, %d chunk addresses", rootsChecked, proofsChecked, addrsChecked)
}

// encodeRawProof renders a § 5.3 body from path elements of ARBITRARY width, so
// a control whose defect is the element width can be put on the wire. It is
// deliberately not EncodeChunkProof, which cannot express the defect.
func encodeRawProof(index int, path [][]byte) []byte {
	out := make([]byte, 0, 8)
	out = append(out, 0x82)
	out = appendCBORUint(out, cborMajorUint, uint64(index))
	out = appendCBORUint(out, cborMajorArray, uint64(len(path)))
	for _, h := range path {
		out = appendCBORUint(out, cborMajorByteStr, uint64(len(h)))
		out = append(out, h...)
	}
	return out
}

// TestSharedVectorsCorruptionControls is the negative half, and it is the half
// that says the verifier is a verifier. A proof checker tested only on valid
// proofs is not tested at all: every control carries ONE deliberate defect over
// an otherwise-valid proof — a flipped chunk byte, a reversed path, a truncated
// or padded path, a mispaired index, a one-bit-wrong root, an nChunks that moves
// where promotion happens, a short sibling, or a malformed § 5.3 body — so a
// rejection can only be about that defect. The generator refuses to emit a
// control its own verifier accepts.
func TestSharedVectorsCorruptionControls(t *testing.T) {
	c := loadChunkProofCorpus(t)

	verifyControls, decodeControls := 0, 0
	for _, ctl := range c.CorruptionControls {
		if ctl.Expect != "reject" {
			t.Fatalf("%s: expect=%q, only \"reject\" is meaningful here", ctl.Name, ctl.Expect)
		}
		switch ctl.Surface {
		case "verify":
			raw := make([][]byte, len(ctl.PathHex))
			path := make([][32]byte, 0, len(ctl.PathHex))
			wrongWidth := false
			for i, h := range ctl.PathHex {
				raw[i] = mustHex(t, h)
				if len(raw[i]) != digestLen {
					wrongWidth = true
					continue
				}
				var e [32]byte
				copy(e[:], raw[i])
				path = append(path, e)
			}
			if wrongWidth {
				// Go's [][32]byte cannot CARRY a wrong-width sibling, so this
				// defect can only reach this implementation over the wire. Assert
				// it is refused THERE — counting it as handled without checking
				// anything would be exactly the hollow control this file exists to
				// avoid.
				if _, _, err := DecodeChunkProof(encodeRawProof(ctl.Index, raw)); err == nil {
					t.Errorf("CONTROL %s (%s) DECODED — a wrong-width sibling was accepted on the wire",
						ctl.Name, ctl.Defect)
				}
				verifyControls++
				continue
			}
			err := VerifyChunkProof(mustAddr(t, ctl.RootHex), ctl.NChunks, ctl.Index,
				mustHex(t, ctl.ChunkHex), path)
			if err == nil {
				t.Errorf("CONTROL %s (%s) VERIFIED — a chunk that must be discarded was accepted",
					ctl.Name, ctl.Defect)
			}
			verifyControls++
		case "decode":
			if _, _, err := DecodeChunkProof(mustHex(t, ctl.ProofBodyHex)); err == nil {
				t.Errorf("CONTROL %s (%s) DECODED — a malformed § 5.3 body was accepted",
					ctl.Name, ctl.Defect)
			}
			decodeControls++
		default:
			t.Fatalf("%s: unknown surface %q", ctl.Name, ctl.Surface)
		}
	}

	total := verifyControls + decodeControls
	if verifyControls == 0 || decodeControls == 0 {
		t.Fatalf("COVERAGE: %d verify-surface and %d decode-surface controls — both halves "+
			"must be exercised", verifyControls, decodeControls)
	}
	if total != c.Counts.CorruptionControls {
		t.Fatalf("COVERAGE: exercised %d controls, corpus declares %d", total, c.Counts.CorruptionControls)
	}
	if total < minControlCount {
		t.Fatalf("COVERAGE: %d controls, floor is %d", total, minControlCount)
	}
	t.Logf("shared corpus: %d controls rejected (%d verify, %d decode)", total, verifyControls, decodeControls)
}

// TestSharedVectorsCoverTheHandCopiedConstants proves the corpus SUPERSEDES the
// hand-copied constants rather than sitting beside them: the n=5 "a".."e" vector
// in the shared file is byte-identical to interopRootB64 / interopProofHex.
//
// Without this, someone could regenerate the corpus with a different tree, watch
// both cross-language suites go green together, and never learn that the fielded
// constants moved. The old pins are the thing the corpus must agree with, not the
// other way round.
func TestSharedVectorsCoverTheHandCopiedConstants(t *testing.T) {
	c := loadChunkProofCorpus(t)

	matched := 0
	for _, v := range c.Vectors {
		if v.Name != "chunkproof_n5_abcde" {
			continue
		}
		if v.RootB64URL != interopRootB64 {
			t.Errorf("corpus n=5 root %s, hand-copied pin %s", v.RootB64URL, interopRootB64)
		}
		if len(v.Proofs) != len(interopProofHex) {
			t.Fatalf("corpus n=5 carries %d proofs, the pin has %d", len(v.Proofs), len(interopProofHex))
		}
		for _, p := range v.Proofs {
			want, ok := interopProofHex[p.Index]
			if !ok {
				t.Fatalf("no hand-copied pin for chunk %d", p.Index)
			}
			if p.ProofBodyHex != want {
				t.Errorf("corpus n=5 chunk %d\ncorpus %s\n   pin %s", p.Index, p.ProofBodyHex, want)
			}
			matched++
		}
	}
	if matched != len(interopProofHex) || matched == 0 {
		t.Fatalf("COVERAGE: matched %d of %d hand-copied proof bodies — the corpus must contain "+
			"the fielded vector, not merely resemble it", matched, len(interopProofHex))
	}
}
