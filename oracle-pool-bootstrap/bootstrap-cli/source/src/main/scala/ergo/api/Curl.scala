package ergo.api

import java.io.{BufferedReader, InputStreamReader}

import org.apache.http.client.methods.{HttpGet, HttpPost}
import org.apache.http.entity.{ContentType, StringEntity}
import org.apache.http.impl.client.HttpClients
import org.apache.http.util.EntityUtils

import scala.language.reflectiveCalls

object Curl {

  private def using[A <: { def close(): Unit }, B](param: A)(f: A => B): B =
    try { f(param) }
    finally { param.close() }

  private val defaultUserAgent = "Mozilla/5.0 (Windows; U; Windows NT 5.1; en-US; rv:1.8.1.1) Gecko/20061204 Firefox/2.0.0.1"

  sealed trait ReqType

  case object Get extends ReqType
  case object PostJsonRaw extends ReqType

  def curl(
      url: String,
      headers: Seq[(String, String)],
      reqType: ReqType,
      params: Seq[(String, String)]
  )(data: Option[String] = None): String = {
    val http = reqType match {
      case Get =>
        val charset = "UTF-8"
        val paramsStr =
          if (params.nonEmpty)
            params
              .map(x => x._1 + "=" + java.net.URLEncoder.encode(x._2, charset))
              .reduceLeft((x, y) => x + "&" + y)
          else ""
        val actualUrl = if (params.isEmpty) url else url + "?" + paramsStr
        val httpGet = new HttpGet(actualUrl)
        httpGet
      case PostJsonRaw =>
        val httpPost = new HttpPost(url)
        val rawJsonData = data.get
        val payload = rawJsonData

        val entity = new StringEntity(payload, ContentType.APPLICATION_JSON)
        httpPost.setEntity(entity)
        httpPost
      case any => throw new Exception(s"Unsupported req type $any")
    }
    headers.foreach {
      case (key, value) =>
        http.addHeader(key, value)
    }

    http.setHeader("User-Agent", defaultUserAgent)
    using(HttpClients.createDefault()) { httpclient =>
      using(httpclient.execute(http)) { resp =>
        val entity = resp.getEntity
        val answer = using(new InputStreamReader(entity.getContent)) { streamReader =>
          using(new BufferedReader(streamReader)) { reader =>
            var line = ""
            var str = ""
            while ({ line = reader.readLine; line != null }) str = str + line
            str
          }
        }
        EntityUtils.consume(entity)
        answer
      }
    }
  }
}
