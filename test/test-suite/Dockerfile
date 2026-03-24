## Copyright (C) 2026 The pgmoneta community
##
## This program is free software: you can redistribute it and/or modify
## it under the terms of the GNU General Public License as published by
## the Free Software Foundation, either version 3 of the License, or
## (at your option) any later version.
##
## This program is distributed in the hope that it will be useful,
## but WITHOUT ANY WARRANTY; without even the implied warranty of
## MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
## GNU General Public License for more details.
##
## You should have received a copy of the GNU General Public License
## along with this program. If not, see <https://www.gnu.org/licenses/>.
FROM docker.io/rockylinux/rockylinux:10

LABEL maintainer="Jesper Pedersen <jesperpedersen.db@gmail.com>"

LABEL summary="PostgreSQL and pgmoneta Composed Image" \
      description="PostgreSQL and pgmoneta Composed Image"

ENV PGVERSION="18"
ENV PGROOT="/usr/pgsql-${PGVERSION}"


RUN mkdir -p /conf-pgmoneta /conf-postgres /pgdata /pgwal /pgmoneta /pglog

# copy config
COPY conf/pgmoneta.conf /conf-pgmoneta/pgmoneta.conf
COPY conf/postgresql.conf /conf-postgres/postgresql.conf
COPY conf/pg_hba.conf /conf-postgres/pg_hba.conf
COPY conf/setup.sql /conf-postgres/setup.sql

# copy executables
COPY usr/bin/run-pgmoneta /usr/bin/run-pgmoneta
COPY usr/bin/run-postgresql /usr/bin/run-postgresql
COPY usr/bin/entry /usr/bin/entry

RUN chmod 755 /usr/bin/run-pgmoneta /usr/bin/run-postgresql /usr/bin/entry

# install needed packages
RUN rpm -Uvh https://dl.fedoraproject.org/pub/epel/epel-release-latest-10.noarch.rpm \
 && rpm -Uvh https://download.postgresql.org/pub/repos/yum/reporpms/EL-10-x86_64/pgdg-redhat-repo-latest.noarch.rpm \
 && dnf -y update \
 && dnf -y install wget tar crontabs \
 && dnf -y install postgresql18 postgresql18-server postgresql18-contrib postgresql18-libs \
 && dnf -y install pgmoneta \
 && dnf -y clean all

RUN useradd -ms /bin/bash pgmoneta \
 && mkdir -p /home/pgmoneta/.pgmoneta \
 && chmod 700 /home/pgmoneta/.pgmoneta \
 && chown pgmoneta:pgmoneta /home/pgmoneta/.pgmoneta

## copy master key
COPY --chown=pgmoneta:pgmoneta ./master.key /home/pgmoneta/.pgmoneta/master.key
RUN chmod 600 /home/pgmoneta/.pgmoneta/master.key

RUN chown -R pgmoneta:pgmoneta /conf-pgmoneta /pgmoneta \
 && chown -R postgres:postgres /conf-postgres /pgdata /pgwal /pglog \
 && chmod 700 /conf-pgmoneta /pgmoneta /conf-postgres /pgdata /pgwal /pglog

VOLUME ["/pgdata","/pgwal","/pgmoneta"]

EXPOSE 5001 9100 5432 5002

ENTRYPOINT ["/usr/bin/entry"]
