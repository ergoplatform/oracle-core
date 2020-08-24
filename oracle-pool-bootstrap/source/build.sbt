name := "oracle-pool-bootstrap"

version := "0.1"

scalaVersion := "2.12.8"

libraryDependencies += "org.apache.httpcomponents" % "httpclient" % "4.3.3"

libraryDependencies += "org.json" % "json" % "20140107"

libraryDependencies += "org.bouncycastle" % "bcprov-jdk14" % "1.65"

scalacOptions += "-feature"

lazy val root = project in file(".")
