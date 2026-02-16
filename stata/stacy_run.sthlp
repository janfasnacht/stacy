{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_run##syntax"}{...}
{viewerjumpto "Description" "stacy_run##description"}{...}
{viewerjumpto "Options" "stacy_run##options"}{...}
{viewerjumpto "Returns" "stacy_run##returns"}{...}
{viewerjumpto "Examples" "stacy_run##examples"}{...}
{title:Title}

{phang}
{bf:stacy run} {hline 2} Execute a Stata script with error detection


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy run} [{it:script}] [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:code(string)}}Inline Stata code{p_end}
{synopt:{opt:directory(string)}}Run Stata in this directory{p_end}
{synopt:{opt:profile}}Include execution metrics{p_end}
{synopt:{opt:quietly}}Suppress output{p_end}
{synopt:{opt:verbose}}Extra output{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy run} execute a stata script with error detection.


{marker options}{...}
{title:Options}

{phang}
{opt cd} change to script's parent directory.

{phang}
{opt code} inline stata code.

{phang}
{opt directory} run stata in this directory.

{phang}
{opt profile} include execution metrics.

{phang}
{opt quiet} suppress output.

{phang}
{opt trace} enable execution tracing at given depth.

{phang}
{opt verbose} extra output.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy run} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(duration_secs)}}Execution time in seconds{p_end}
{synopt:{cmd:r(error_count)}}Number of errors detected{p_end}
{synopt:{cmd:r(exit_code)}}Exit code (0=success){p_end}
{synopt:{cmd:r(success)}}Whether script succeeded (1=yes, 0=no){p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(log_file)}}Path to log file{p_end}
{synopt:{cmd:r(script)}}Path to script{p_end}
{synopt:{cmd:r(source)}}'file' or 'inline'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy run}{p_end}


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
