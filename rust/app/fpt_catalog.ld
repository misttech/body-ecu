/* Copyright 2026 Mist Tecnologia LTDA. All rights reserved. */

/* The Forkpoint SDK's assertion catalog: one record per property the firmware states,
 * which `fpt catalog` lists and `fpt explore` judges against. Not loaded: it is for the
 * host, not the CPU. */
.fpt_catalog (INFO) : { KEEP(*(.fpt_catalog)) }
