{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_test##syntax"}{...}
{viewerjumpto "Description" "stacy_test##description"}{...}
{viewerjumpto "Options" "stacy_test##options"}{...}
{viewerjumpto "Returns" "stacy_test##returns"}{...}
{viewerjumpto "Examples" "stacy_test##examples"}{...}
{title:Title}

{phang}
{bf:stacy test} {hline 2} Run tests


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy test} {it:test} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:filter(string)}}Filter tests by pattern{p_end}
{synopt:{opt:list}}List tests without running{p_end}
{synopt:{opt:parallel}}Run tests in parallel{p_end}
{synopt:{opt:quiet}}Suppress progress output{p_end}
{synopt:{opt:verbose}}Show full log context for failures{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy test} run tests.


{marker options}{...}
{title:Options}

{phang}
{opt filter} filter tests by pattern.

{phang}
{opt list} list tests without running.

{phang}
{opt parallel} run tests in parallel.

{phang}
{opt quiet} suppress progress output.

{phang}
{opt verbose} show full log context for failures.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy test} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(duration_secs)}}Total execution time in seconds{p_end}
{synopt:{cmd:r(failed)}}Number of failed tests{p_end}
{synopt:{cmd:r(passed)}}Number of passed tests{p_end}
{synopt:{cmd:r(skipped)}}Number of skipped tests{p_end}
{synopt:{cmd:r(success)}}Whether all tests passed (1=yes, 0=no){p_end}
{synopt:{cmd:r(test_count)}}Total number of tests{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(test_names)}}Comma-separated test names (for --list){p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy test}{p_end}


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
