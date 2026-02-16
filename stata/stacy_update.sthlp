{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_update##syntax"}{...}
{viewerjumpto "Description" "stacy_update##description"}{...}
{viewerjumpto "Options" "stacy_update##options"}{...}
{viewerjumpto "Returns" "stacy_update##returns"}{...}
{viewerjumpto "Examples" "stacy_update##examples"}{...}
{title:Title}

{phang}
{bf:stacy update} {hline 2} Update packages to latest versions


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy update} {it:packages} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:dryrun}}Show what would be updated without making changes{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy update} update packages to latest versions.


{marker options}{...}
{title:Options}

{phang}
{opt dry_run} show what would be updated without making changes.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy update} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(dry_run)}}Whether this was a dry run (1=yes, 0=no){p_end}
{synopt:{cmd:r(failed)}}Number of packages that failed to update{p_end}
{synopt:{cmd:r(total)}}Total packages checked{p_end}
{synopt:{cmd:r(updated)}}Number of packages updated{p_end}
{synopt:{cmd:r(updates_available)}}Number of packages with updates available{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(status)}}'success', 'partial', or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy update}{p_end}


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
