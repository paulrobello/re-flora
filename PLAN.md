# Coding Plans

## Erase special feature and extension usage

Right now this project will crash on MacOS with M4 Pro chip, with the following error message:

```plaintext
[16:46:58.554 INFO re_flora::app::core] sum: 3
Failed to grab cursor: NotSupported(NotSupportedError)
--- Physical Device Evaluation Report ---
+--------------+----------------+-------------+-------------+-----------------------+
| Device       | Type           | Memory (MB) | Suitability | Reason                |
+===================================================================================+
| Apple M4 Pro | INTEGRATED_GPU | 49152.00    | Suitable    | All requirements met. |
+--------------+----------------+-------------+-------------+-----------------------+

--- Suitable Physical Devices ---
+--------------+----------------+-------------+-------+-----------+
| Device       | Type           | Memory (MB) | Score | Selected? |
+=================================================================+
| Apple M4 Pro | INTEGRATED_GPU | 49152.00    | 242   | Yes       |
+--------------+----------------+-------------+-------+-----------+

--- Queue Family Analysis for Selected Device ---
+--------------------+----------+---------+---------+----------+----------------+
| Queue Family Index | Graphics | Present | Compute | Transfer | Sparse Binding |
+===============================================================================+
| 0                  | Yes      | Yes     | Yes     | Yes      |                |
|--------------------+----------+---------+---------+----------+----------------|
| 1                  | Yes      | Yes     | Yes     | Yes      |                |
|--------------------+----------+---------+---------+----------+----------------|
| 2                  | Yes      | Yes     | Yes     | Yes      |                |
|--------------------+----------+---------+---------+----------+----------------|
| 3                  | Yes      | Yes     | Yes     | Yes      |                |
+--------------------+----------+---------+---------+----------+----------------+

--- Selected Queue Family Indices ---
+--------------------------------------+--------------------+
| Queue Type                           | Queue Family Index |
+===========================================================+
| General (Graphics, Present, Compute) | 0                  |
|--------------------------------------+--------------------|
| Dedicated Transfer (if available)    | 1                  |
+--------------------------------------+--------------------+
[16:46:58.636 INFO re_flora::vkn::context::physical_device] Selected physical device: Apple M4 Pro
+----------------+----------------+
| Desired        | Using          |
+=================================+
| B8G8R8A8_SRGB  | B8G8R8A8_SRGB  |
|----------------+----------------|
| SRGB_NONLINEAR | SRGB_NONLINEAR |
+----------------+----------------+
[16:46:59.082 INFO re_flora::vkn::swapchain] Swapchain present mode: FIFO
```

## Grass System

We already have grasses, as drawn by flora system. However the density of the grasses is not enough. We need to increase the density of the grasses.

Also, the grasses are of the same height, which needs some more variations to make it more realistic.

We have two plans for this:

1. Create different types of grasses model, with different heights, and spawn them randomly.
2. Use a single grass model, but degenerate some of the topped voxels based on the real height, in flora.vert.

Analyze which is better, the first one seems has better perf, but the second one is more flexible.

Write your thoughts here.
