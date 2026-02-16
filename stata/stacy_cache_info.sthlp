{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_cache_info##syntax"}{...}
{viewerjumpto "Description" "stacy_cache_info##description"}{...}
{viewerjumpto "Options" "stacy_cache_info##options"}{...}
{viewerjumpto "Returns" "stacy_cache_info##returns"}{...}
{viewerjumpto "Examples" "stacy_cache_info##examples"}{...}
{title:Title}

{phang}
{bf:stacy cache_info} {hline 2} Show cache statistics


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy cache_info} 

{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy cache_info} show cache statistics.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy cache_info} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(cache_exists)}}Whether cache file exists (1=yes, 0=no){p_end}
{synopt:{cmd:r(entry_count)}}Number of cached entries{p_end}
{synopt:{cmd:r(size_bytes)}}Approximate size in bytes{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(cache_path)}}Path to cache file{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy cache_info}{p_end}


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
