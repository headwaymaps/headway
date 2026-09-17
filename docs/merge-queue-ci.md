# Reusing CI results for a serial merge queue

## Goal

Avoid rerunning the full CI suite when a merge queue contains one pull request
whose required checks already passed against the exact current `main` commit.
The merge queue must still receive a successful `All checks` result for its
synthesized merge commit.

This is an optimization for the normal serial workflow. It is not a general
replacement for merge-group validation.

## Fast-path requirements

The `merge_group` workflow may reuse pull-request results only when all of the
following can be proved:

1. The merge group contains exactly one pull request.
2. The synthetic merge commit has exactly two parents: the merge group's
   `base_sha` and the pull request's head SHA.
3. The pull request is open, targets the merge group's `base_ref`, and its
   recorded base SHA equals the merge group's `base_sha`.
4. The pull request's head SHA equals the non-base parent of the synthetic
   merge commit.
5. The required `All checks` check run completed successfully on that head SHA.
   The check must be from GitHub Actions and from this repository's Build Check
   workflow.

Together, these conditions establish that the queue commit changes neither the
base nor the code that passed pull-request CI. A merge conflict cannot satisfy
them because GitHub would not create the synthetic merge commit.

## Fail closed

Any missing API data, unexpected commit topology, multiple queued pull
requests, changed base SHA, missing check, failed check, or API error must run
the existing full `merge_group` suite. A successful reuse result must never be
returned based on an assumption or a best-effort lookup.

This also means that any change to `main`, any PR added ahead of the current
PR, and every merge batch automatically receives full CI.

## Workflow shape

Keep `pull_request` and `merge_group` as workflow triggers. Add a small,
unprivileged `merge_queue_gate` job that runs only for `merge_group` and emits
one output:

```text
reuse=true | false
```

For a pull request, run the current full jobs unchanged. For a merge group:

* if `reuse=false`, run the current full jobs unchanged;
* if `reuse=true`, skip the full jobs and run a replacement `All checks` job
  that reports success only after `merge_queue_gate` completes successfully.

The aggregate check must retain the exact required-check name, `All checks`.
The current aggregate job must treat skipped full jobs as acceptable only when
the gate explicitly produced `reuse=true`; skipped jobs remain failures in all
other cases.

The gate should use the GitHub REST API with the workflow token, read the
merge-group `head_sha`, `base_sha`, and `base_ref`, inspect the synthetic
commit's parents, locate the matching pull request, and inspect check runs on
the PR head. It needs read-only `contents`, `pull-requests`, and `checks`
permissions.

## Limits and observability

* Start with merge groups limited to one pull request. Do not attempt to infer
  that multiple PRs were independently tested together.
* Log a concise reason for every `reuse=false` decision: changed base,
  multiple parents, missing matching PR, or missing required check.
* Add the gate decision, PR number, PR head SHA, and base SHA to the job
  summary. This makes any accidental fast-path decision auditable.
* Keep the existing workflow concurrency cancellation. If `main` advances or
  the queue recomposes, the stale merge group must be cancelled and evaluated
  again.

## Expected benefit

For an up-to-date, serial PR, the merge-group run becomes an API lookup plus
one aggregate check instead of repeating the roughly 24-minute full workflow.
It provides no acceleration when the base changes or the queue contains more
than one PR, which is intentional: those are exactly the cases where
merge-group testing adds unique value.
