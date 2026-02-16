{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_cache_clean##syntax"}{...}
{viewerjumpto "Description" "stacy_cache_clean##description"}{...}
{viewerjumpto "Options" "stacy_cache_clean##options"}{...}
{viewerjumpto "Returns" "stacy_cache_clean##returns"}{...}
{viewerjumpto "Examples" "stacy_cache_clean##examples"}{...}
{title:Title}

{phang}
{bf:stacy cache_clean} {hline 2} Remove cached entries


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy cache_clean} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:olderthan(integer)}}Remove entries older than N days{p_end}
{synopt:{opt:quiet}}Suppress output{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy cache_clean} remove cached entries.


{marker options}{...}
{title:Options}

{phang}
{opt older_than} remove entries older than n days.

{phang}
{opt quiet} suppress output.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy cache_clean} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(entries_remaining)}}Number of entries remaining{p_end}
{synopt:{cmd:r(entries_removed)}}Number of entries removed{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(status)}}'success' or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy cache_clean}{p_end}


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
