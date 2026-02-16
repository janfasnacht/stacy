{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_task##syntax"}{...}
{viewerjumpto "Description" "stacy_task##description"}{...}
{viewerjumpto "Options" "stacy_task##options"}{...}
{viewerjumpto "Returns" "stacy_task##returns"}{...}
{viewerjumpto "Examples" "stacy_task##examples"}{...}
{title:Title}

{phang}
{bf:stacy task} {hline 2} Run tasks from stacy.toml


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy task} {it:task} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:frozen}}Fail if lockfile doesn't match stacy.toml{p_end}
{synopt:{opt:list}}List available tasks{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy task} run tasks from stacy.toml.


{marker options}{...}
{title:Options}

{phang}
{opt frozen} fail if lockfile doesn't match stacy.toml.

{phang}
{opt list} list available tasks.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy task} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(duration_secs)}}Total execution time in seconds{p_end}
{synopt:{cmd:r(exit_code)}}Exit code (0=success){p_end}
{synopt:{cmd:r(failed_count)}}Number of failed scripts{p_end}
{synopt:{cmd:r(script_count)}}Number of scripts executed{p_end}
{synopt:{cmd:r(success)}}Whether task succeeded (1=yes, 0=no){p_end}
{synopt:{cmd:r(success_count)}}Number of successful scripts{p_end}
{synopt:{cmd:r(task_count)}}Number of tasks defined{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(task_name)}}Name of the task{p_end}
{synopt:{cmd:r(task_names)}}Comma-separated task names (for --list){p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy task}{p_end}


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
