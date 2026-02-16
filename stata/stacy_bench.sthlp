{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_bench##syntax"}{...}
{viewerjumpto "Description" "stacy_bench##description"}{...}
{viewerjumpto "Options" "stacy_bench##options"}{...}
{viewerjumpto "Returns" "stacy_bench##returns"}{...}
{viewerjumpto "Examples" "stacy_bench##examples"}{...}
{title:Title}

{phang}
{bf:stacy bench} {hline 2} Benchmark script execution


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy bench} {it:script} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:nowarmup}}Skip warmup runs{p_end}
{synopt:{opt:quiet}}Suppress progress output{p_end}
{synopt:{opt:runs(integer)}}Number of measured runs{p_end}
{synopt:{opt:warmup(integer)}}Number of warmup runs{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy bench} benchmark script execution.


{marker options}{...}
{title:Options}

{phang}
{opt no_warmup} skip warmup runs.

{phang}
{opt quiet} suppress progress output.

{phang}
{opt runs} number of measured runs.

{phang}
{opt warmup} number of warmup runs.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy bench} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(max_secs)}}Maximum execution time in seconds{p_end}
{synopt:{cmd:r(mean_secs)}}Mean execution time in seconds{p_end}
{synopt:{cmd:r(measured_runs)}}Number of measured runs{p_end}
{synopt:{cmd:r(median_secs)}}Median execution time in seconds{p_end}
{synopt:{cmd:r(min_secs)}}Minimum execution time in seconds{p_end}
{synopt:{cmd:r(stddev_secs)}}Standard deviation in seconds{p_end}
{synopt:{cmd:r(success)}}Whether all runs succeeded (1=yes, 0=no){p_end}
{synopt:{cmd:r(warmup_runs)}}Number of warmup runs{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(script)}}Path to benchmarked script{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy bench}{p_end}


{marker author}{...}
{title:Author}

{pstd}
Jan Fasnacht{p_end}
{pstd}
{browse "https://github.com/janfasnacht/stacy":github.com/janfasnacht/stacy}{p_end}


{marker also_see}{...}
{title:Also see}

{psee}
{space 2}Help:  {helpb stacy}
{p_end}
